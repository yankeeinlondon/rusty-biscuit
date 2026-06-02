# Stable Broker Pane Lifetime for Level 2 Tests

## Context

### The performance problem

Darkmatter's Level 2 test binaries (`darkmatter-cli::level2_layout`, 48 tests; `darkmatter-cli::level2_errors`, 3 tests) consistently land in the "slow" tier — most tests take 7–11 seconds each. The `darkmatter` Level 2 layout suite alone runs 39+ seconds in direct nextest mode, longer under `just test-l2`.

Profiling a single test shows the time is not in `md` itself (~200–500 ms cold) but in harness overhead:

| Component | Cost per test |
|---|---|
| `wezterm cli send-text` subprocess (1×) | ~100–200 ms |
| `wezterm cli get-text` subprocess (~3–5× per polling cycle) | ~300–1000 ms |
| `settle()` hard sleep (× 2 round-trips: `clear`, then `md`) | 400 ms |
| Post-sentinel sleep (250 ms × 2 round-trips) | 500 ms |
| `run_with_timeout` 50 ms inter-poll sleeps inside each subprocess wait | ~150 ms |
| **Harness-only floor** | **~1.5–3 s** |

The full `md` command runs twice through this pipeline per test (a separate `run_with_sentinel(harness, "clear")` then `run_with_sentinel_env(harness, "md ...", env)`). Cutting harness overhead in half would push most tests below the 5 s SLOW threshold.

### Why we cannot just optimize harness timings today

A May 2026 attempt to reduce `settle()` from 200 ms → 50 ms, fold `clear` into the same wrapped command as `md`, and tighten `run_with_timeout`'s inter-poll sleep all *worked* under direct `cargo nextest run -p darkmatter-cli -E 'test(/level2_/)'` (51 tests passed, total runtime dropped from 39.3 s to 28.5 s — about 25 % faster). Under `just test-l2` they broke `level2_ul_color_inherits_into_li_body`.

But `just test-l2` was *already* failing in baseline before any change. Multiple stash/run cycles produced failures like:

```
TRY 4 FAIL [   0.067s] (1/51) darkmatter-cli::level2_errors
                            level2_error_excerpt_contains_gutter_and_dimming
TRY 4 FAIL [   0.181s] (2/9)  darkmatter::level2_render_tree_terminal
                            level2_page_no_double_blank_rows_between_code_blocks
TRY 4 FAIL [   0.080s] (49/51) darkmatter-cli::level2_layout
                            level2_table_center_alignment_indents_more_than_left
```

All under 1 s. These are not real assertion failures — they are panics on harness attach because the broker-spawned WezTerm pane has died between test binaries.

This means **`just test-l2` is unreliable in baseline**, and that unreliability blocks any performance work that *might* break a test by changing timing. Every attempt to validate a speedup is contaminated by intermittent broker-pane death.

### What the broker is supposed to do

`_test_l2` (in `just/devops.just`) pre-spawns a single shared pane via `biscuit-harness-broker`, exports its id through `BISCUIT_SHARED_WEZTERM_ID` (and analogous vars for other backends), then runs nextest. Inside each test binary, `WezTermHarness::shared_or_spawn()` either *attaches* to the broker pane (when the env var is set) or *spawns its own* (when not).

Goal: every Level 2 test binary in a given run shares one pane, no pane is created or destroyed except by the broker, and pane teardown happens once at the end of `_test_l2` via a bash `trap`.

Observed failure mode: the pane gets killed (or marked dead) mid-run, and subsequent tests panic on attach.

### How fixing this unblocks performance work

Once `just test-l2` is *itself* deterministic, every speedup change becomes verifiable. We can land the changes that were stashed during the May 2026 attempt:

1. `settle()` 200 ms → 50–100 ms (saves ~300 ms per round-trip).
2. Fold `clear` into the `md` wrapped command (saves one full round-trip — multiple subprocesses + settle + poll + post-sentinel sleep, ~500–1000 ms per test).
3. `run_with_timeout` poll 50 ms → 10 ms (saves ~100 ms per test).
4. Post-sentinel sleep 250 ms → 100 ms (saves ~150 ms per test).

Combined these conservatively cut **~1 s** off each Level 2 test. With 51 tests in `darkmatter-cli` and another 9 in `darkmatter`, that is **~60 s** off a full `just test-l2` run.

This spec covers step 0 — making the test harness reliable enough to trust the speedups.

---

## Goals

1. `just test-l2` (using the broker) runs deterministically: same pass/fail result on three consecutive runs against an unchanged source tree.
2. The broker pane survives for the duration of the entire `_test_l2` invocation (not just a single test binary).
3. When a test binary finishes, the next binary attaches to the *same* broker pane, not a fresh spawn.
4. Pane teardown is owned exclusively by `_test_l2`'s trap; no test binary, harness destructor, or atexit handler kills the broker pane mid-run.
5. The fix is observable: a CI-style "broker health" probe can be run before and after each test binary to detect mid-run pane death.

## Non-Goals

- Fixing `just test-l3` (OS-level keyboard injection) or `just test-browser`. Both have their own harness paths.
- Reducing `wezterm cli` subprocess overhead (that is the follow-up performance work).
- Replacing WezTerm with a different terminal backend.
- Eliminating the broker pattern (it is the right architecture; we are just hardening it).

## Investigation Phase

Before designing the fix, we need to know *why* the pane dies. Likely suspects, in order of plausibility:

1. **A test binary's `SharedHarness` cleanup atexit handler kills the broker pane.** `SharedHarness` registers a `libc::atexit` to kill its pane on process exit so the workspace does not leak. If the broker-spawned pane id matches what `shared_or_spawn` returns, the atexit fires on the first binary's exit and removes the pane the second binary then tries to attach to. This is the highest-suspicion failure mode.
2. **`cleanup_stale_wezterm_panes` runs too aggressively.** The cleanup function (`biscuit-test-harness/src/wezterm.rs`) removes untagged panes from the `biscuit-bg` workspace when the count exceeds a threshold or `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_WEZTERM=1` is set. The broker pane might match an untagged-cleanup heuristic.
3. **WezTerm itself kills idle panes** under some `idle_workers_timeout` or workspace-level setting. Less likely but possible if WezTerm config has changed.
4. **PID-based cleanup heuristic mismatch.** Tagged panes use `biscuit-test-pane-<pid>-<pane_id>`. The broker's pid is *different* from each test binary's pid. If a test binary's cleanup pass treats "pane tagged with a non-current pid" as dead-process garbage, it removes the broker pane.

### Required investigation tasks

1. Add `tracing` instrumentation (or `eprintln!` behind `BISCUIT_HARNESS_DEBUG`) to every `kill_pane` / `kill_wezterm_pane` / `kill_pane_by_id` call site, logging the caller, the target pane id, and the broker pane id from `BISCUIT_SHARED_WEZTERM_ID`. Run `just test-l2` and confirm which call sites fire against the broker pane.
2. Capture `wezterm cli list --format json` before each test binary and after each test, attribute pane-death timing.
3. Verify: does the broker pane have any tag at all? `biscuit-harness-broker` source needs auditing. If it tags the pane with the broker's own pid, every test-binary cleanup pass will see "pid does not exist in *my* process tree" and may delete it.

## Proposed Design

### 1. Broker-owned tagging that survives PID mismatch

The broker pane must be marked with a tag that all test binaries recognize as "do not touch, owned by the broker." Proposed tag format:

```
biscuit-broker-pane-<broker-pid>-<pane-id>
```

Distinct from the per-test tag `biscuit-test-pane-<test-pid>-<pane-id>`. All cleanup routines must early-out on `biscuit-broker-pane-*` regardless of whose pid they see.

### 2. `SharedHarness` must skip cleanup of an *attached* pane

`SharedHarness<T>::get_or_init` already distinguishes "attach to pre-existing pane" from "spawn new pane" by inspecting `BISCUIT_SHARED_<BACKEND>_ID`. When the value is "attach", the `libc::atexit` handler registered for that `SharedHarness` instance must be a no-op. Today it likely runs `kill_pane_by_id` unconditionally.

Concretely: `WezTermHarness::shared_or_spawn` should set an internal `owns_pane: bool` flag — `false` when attached, `true` when freshly spawned — and the `Drop` / `atexit` path must consult it.

### 3. `cleanup_stale_wezterm_panes` must whitelist the active broker pane

The stale-pane sweep needs to read `BISCUIT_SHARED_WEZTERM_ID` (and equivalents) and skip any pane id listed there. Today it sweeps any untagged pane in the `biscuit-bg` workspace once the count exceeds `LEGACY_BACKGROUND_PANE_LIMIT` or the explicit sweep env var is set. If a stale-broker-pane from a previous crashed run is sitting in the workspace, this sweep is the correct way to remove it — but it must not remove the *currently active* broker pane.

### 4. Broker health probe in `_test_l2`

Wrap the `cargo nextest run` invocation in `_test_l2` with pre- and post-checks:

```bash
broker_alive() {
    wezterm cli list --format json \
        | jq --arg id "${BISCUIT_SHARED_WEZTERM_ID}" \
            '[.[] | select((.pane_id | tostring) == $id)] | length > 0'
}

trap 'broker_alive | grep -q true || echo "::warning::broker pane died mid-run"' EXIT
```

Or, more strictly, fail the run if the broker pane dies before all binaries have finished. The diagnostic by itself is enough to convert flaky failures into actionable signals.

### 5. Optional: broker heartbeat

`biscuit-harness-broker` can fork a background thread that polls `wezterm cli list` every 5 s and re-spawns the pane if it disappears, updating `BISCUIT_SHARED_WEZTERM_ID` via a sidecar file rather than env (since env is fixed at process start). This is the most invasive option and should probably wait until #1–#4 are landed and measured.

## Acceptance Criteria

1. `just test-l2` run 5 times in succession against an unchanged tree produces 5 passing runs.
2. No `0.0X s` panics in nextest output (those indicate harness attach failures).
3. `wezterm cli list` before and after each `_test_l2` invocation shows: zero panes in `biscuit-bg` workspace *before*, zero *after*.
4. Adding `BISCUIT_HARNESS_DEBUG=1` produces a log showing exactly one pane spawn (by the broker) and one pane kill (by the broker's exit trap), no test-binary-initiated kills against the broker pane id.

## Risks

- **Heisenbug**: the pane-death cause may shift once instrumentation is added. Plan for at least one round of "add logging, run, find suspect, fix, repeat."
- **Other backends**: the same audit needs to happen for `KittyHarness` and `TmuxHarness` cleanup paths. They may not exhibit the bug today only because Level 2 tests do not exercise them as heavily.
- **CI environment differences**: macOS, Linux, and CI runners may have different `wezterm cli` socket semantics. Verify the fix on each.

## Estimated Effort

- Investigation (instrumentation + repro): 1 day
- Fixes (#1–#4): 2 days
- Verification (multi-run stability, CI loop): 1 day
- **Total: ~4 days**

## Follow-Up Work Unblocked by This Spec

Once `just test-l2` is reliable, ship the four harness-timing reductions identified in May 2026:

1. `settle()` 200 ms → 50–100 ms.
2. Fold `clear` into the wrapped `md` command (single round-trip per test).
3. `run_with_timeout` poll 50 ms → 10 ms.
4. Post-sentinel sleep 250 ms → 100–150 ms.

Combined target: most Level 2 tests under 5 s (drop out of the SLOW tier), total `just test-l2` runtime ≤ 25 s for `darkmatter-cli`'s 51 tests.
