---
status: implemented
created: 2026-08-05
---

# macOS CI Timing Margins in biscuit-terminal

## Summary

Two `biscuit-terminal` tests fail on the `macos-latest` CI runner and on no
developer machine:

```
LEAK-FAIL [ 0.124s] ( 869/2942) biscuit-terminal components::prose::tests::standalone_code_block_emits_no_restoration_escape
     FAIL [ 0.329s] (2069/2942) biscuit-terminal::level1_cursor cursor_position_parses_csi_r_reply
```

Neither is caused by the PR they were observed in (#35, which touches only
`tree-hugger`). `cursor_position_parses_csi_r_reply` fails identically on
`main`.

The cursor failure is a demonstrated wall-clock race. The `LEAK-FAIL` has a
different evidence level: the test's reachable path owns no child process or
inherited pipe handle, so delayed observation of closed output handles under
runner contention is the leading explanation, but one CI occurrence does not
prove the scheduler mechanism. The CI runner image is `macos-26-arm64`, a
3-core machine; nextest defaults to `-j = ncpu`, so CI runs three test processes
on three cores while a 16-core developer host runs sixteen on sixteen. The same
nominal configuration produces different scheduling scarcity.

This is not a "flaky test" in the sense of an unlucky coin flip.
`cursor_position_parses_csi_r_reply` failed in *both* macOS runs examined, at
the identical position in the run (2069/2942). On three cores it fails nearly
always; on sixteen it fails nearly never.

## Review Findings

The proposed direction is sound, subject to these implementation constraints:

1. **Generalize the shared answer type.** Reusing an `OscAnswer` for a CPR
   exchange makes the abstraction and its documentation false. Rename it to a
   protocol-neutral name such as `ProbeAnswer`, update its documentation, and
   update the existing OSC-cache call sites in the same change.
2. **Drive each probe through a terminal result marker.** Observing `ESC[6n` is
   the trigger for the reply, not the completion condition. Each cursor test
   must continue draining until its `cursor_position=...` or
   `cursor_timeout=...` line is observed so the spawned child exits before the
   PTY session is dropped.
3. **Treat the 10ms poll interval as nominal, not guaranteed.** The new design
   removes the fixed 80ms delay and process-start timing from the race, but the
   driver thread can still be descheduled between observing the query and
   writing the reply. The improvement is a substantially larger response
   budget, not a hard 90ms margin.
4. **Limit and order the nextest override deliberately.** The failure has only
   been observed on macOS, so the new 1s override should include a macOS host
   filter. It must appear after existing, more-specific leak-timeout overrides;
   nextest uses the first matching override for each setting, and the existing
   `browser_` 5s exception must retain precedence.
5. **Do not encode test counts as a contract.** Counts vary with features and
   tier filters and will drift. Scope validation must assert that `kind(lib)`
   selects the library unit-test binary and no integration binaries, while
   treating the observed count as diagnostic output only.
6. **Retain the OSC timing debt explicitly.** The OSC tests have ample measured
   margin today, so migrating them is not required for this fix. Their fixed
   sleeps remain temporally coupled to the implementation and could become
   unsafe if terminal-app discovery becomes faster.

## Defect 1 — `cursor_position_parses_csi_r_reply`

### Current State

`level1_cursor.rs` spawns the `discovery_probe` example in a PTY, waits a fixed
`sleep(80ms)`, then writes a manufactured CPR reply (`ESC[12;34R`) and asserts
the probe parsed it.

The probe calls `cursor_position()`, which allows the terminal **100ms** to
respond (`discovery/cursor_position.rs:35`). The test's sleep and the probe's
deadline are two independent clocks, and they overlap far less than the numbers
suggest.

### Measurement

Instrumented on this host (16-core macOS, debug build, load average ~8.7),
five trials, timing from immediately before the `spawn` call:

| quantity | measured |
|---|---|
| `expectrl::Session::spawn` returns | 120.4 – 129.1 ms |
| probe emits `ESC[6n` | 126.3 – 132.9 ms |
| **D** = spawn-return → query emitted | **3.6 – 8.7 ms** |
| reply observed → probe prints result | 1.2 – 2.4 ms |

The test starts its sleep when `spawn` *returns*, so process startup cancels
out of the arithmetic and the surviving term is small:

```
test writes reply at:   spawn_return + sleep
probe deadline expires: spawn_return + D + 100ms

failure when:           sleep > D + 100ms
margin at sleep=80ms:   D + 20ms  =  23.6 – 28.7 ms
```

### Evidence

Predicted cliff is `D + 100` = 103.6 – 108.7 ms. Sweeping the pre-reply sleep
against the unmodified test:

| sleep | result |
|---|---|
| 80 ms | PASS |
| 95 ms | PASS |
| 105 ms | PASS |
| 115 ms | FAIL |
| 130 ms | FAIL |

The cliff lands exactly where the model predicts. A ~26ms scheduling delay on
the test thread's `sleep(80ms)` is sufficient to produce the CI symptom, and
26ms of wake-up latency on a saturated 3-core machine is unremarkable.

CI occurrences, both at run position 2069/2942 — deep inside a saturated batch:

- run `30812737709`, job `91687530511` (PR #35) — `FAIL [0.329s]`
- run `30791562395`, job `91622553939` (`main`) — `FAIL [0.468s]`

Failure message in both: `expected parsed cursor position in output, got:
cursor_position=None`.

### `#[serial]` Offers No Protection

The three tests in `level1_cursor.rs` carry `#[serial]`. nextest runs each test
in its own process, so `serial_test`'s in-process mutex does nothing across
tests. This is already documented in the repository at
`just/devops.just:1546`. The attribute should not be read as isolation.

### `level1_osc_queries.rs` Is Not Currently Failing

`level1_osc_queries.rs` uses the same `sleep(80ms)` pattern against the same
100ms `DEFAULT_TIMEOUT`, so it appears to carry an identical defect. It does
not. `query_osc_color_with_timeout` calls `get_terminal_app()` *before* writing
its query, so the probe does not emit for far longer after spawn returns.

Sweeping the OSC pre-reply sleep the same way:

| sleep | 80 | 105 | 130 | 160 | 200 | 260 |
|---|---|---|---|---|---|---|
| result | PASS | PASS | PASS | PASS | PASS | PASS |

Still passing at 260ms implies `D_osc > 160ms` — an order of magnitude more
slack than the cursor path. `cursor_position()` performs no detection before
writing its query; it goes straight to `open("/dev/tty")` and writes. It is the
fastest test to reach its query and therefore the one with the least slack.
The OSC tests currently have extra margin because this path is slow.

**No change is required in `level1_osc_queries.rs` for this fix.** This is not a
claim that fixed sleeps are a durable synchronization contract; query-driven
replies remain an appropriate follow-up if this path is changed.

### Proposed Fix

Reply when the query is *observed* rather than after a fixed sleep. The
repository already has the correct control flow in `drive_probe` /
`OscAnswer` in
`biscuit-terminal/lib/tests/common/pty.rs` — used by
`level1_terminal_osc_cache.rs`. It reads the master stream, writes each answer
once when its query first appears, and returns when a marker is seen.

Before sharing it with the cursor tests, rename `OscAnswer` to a
protocol-neutral `ProbeAnswer` and update its OSC-specific documentation. This
is a direct generalization of an existing one-shot query/reply abstraction, not
a second PTY driver.

All three tests in `level1_cursor.rs` route through one local helper. The helper
answers only after observing `ESC[6n`, then continues draining until the
probe's result line is present. Test 1 does not need a valid CPR to prove query
emission, but it should still send one and wait for `cursor_position=...` so the
probe exits cleanly rather than being abandoned immediately after its query is
seen.

The nominal detection delay is the `drive_probe` poll interval (10ms), leaving
most of the probe's 100ms budget available. Scheduler wake-up latency and the
reply write still consume part of that budget, so the design must not claim a
hard 90ms margin. Process spawn latency is removed from the response timing.

## Defect 2 — Spurious `LEAK-FAIL` on the prose unit test

### Current State

`components::prose::tests::standalone_code_block_emits_no_restoration_escape`
reported `LEAK-FAIL`, with nextest's own output recording:

```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1882 filtered out
(test failed: exited with code 0, but leaked handles)
```

The assertions passed. `LEAK-FAIL` means nextest did not observe EOF on the
child's stdout/stderr pipes within the 100ms `leak-timeout` window after the
child exited.

### Analysis

The test spawns nothing. It renders `Prose::new("```\ncode\n```")` through
`render_optimistic`, which builds `Terminal::new_optimistic` — a plain struct
literal (`terminal.rs:419`) that performs no capability discovery. The body is
a string comparison; nextest timed it at 0.00s inside a 0.124s process.

The only `Command::new` reachable from the prose module is `find_git_root` in
`components/prose/styles.rs:148`, on the link-href resolution path. A fenced
code block contains no links, and `Command::output()` pipes stdout/stderr and
waits, so even if reached it could not hold the test's pipes.

Nextest waits for the test process's stdout and stderr handles to close after
the process exits. Because this test's reachable path creates no descendant
that can inherit those handles, the result is inconsistent with a genuine leak
originating in the test. Delayed output-handle observation under contention is
therefore the leading explanation, not a proven root cause.

This matches five precedents already documented in `.config/nextest.toml`:
`browser_`, `preflight_graph_*`, `worktree`, `biscuit-speaks-cli`, and
`claudine-cli` L2. Two of those (`preflight_graph_*`, `worktree`) are likewise
tests that provably spawn nothing.

### Proposed Fix

A scoped `leak-timeout` grace window, following the established pattern, in
**both** `profile.default` and `profile.ci`:

```toml
[[profile.default.overrides]]
platform = { host = 'cfg(target_os = "macos")' }
filter = 'package(biscuit-terminal) & kind(lib)'
leak-timeout = { period = "1s", result = "fail" }
```

Use the equivalent block under `profile.ci`. Place both blocks after the
existing leak-timeout exceptions. Nextest applies the first matching override
for each setting, so this ordering preserves the existing 5s `browser_` grace
for any matching library test.

`kind(lib)` was verified to select the library unit-test binary and zero
integration binaries. The exact test count is feature- and tier-dependent and
is not part of the contract. The strict 100ms window stays in force for the
PTY-spawning integration tests, and for all `biscuit-terminal` tests on Linux
and Windows.

`result = "fail"` is retained deliberately. Downgrading to a warning would hide
genuine leaks; 1s only absorbs teardown and scheduling lag.

Scoping to the single test name was rejected because the evidence points to
test-binary output-handle observation rather than behavior unique to that test.
A name-scoped exception would encode the observed victim without addressing
the inferred failure domain.

## Reproduction Limits

Neither defect reproduces on a 16-core host. Added spin load did not reproduce
the scheduling scarcity of the 3-core runner while idle cores remained
available, so local load generation is not an equivalent environment.

Attempts made:

- cursor tests, 15 runs at `-j4` under 48-way spin load, `--retries 0` —
  15/15 pass, both before and after the fix
- lib unit tier, 8 runs at `-j16` under 48-way spin load, `--retries 0` —
  zero `LEAK` lines

For the cursor defect, the margin sweep replaces the local repro: a 26ms margin
against a 100ms deadline is demonstrably unsafe on a 3-core runner whether or
not it can be triggered on a 16-core one. The `LEAK-FAIL` explanation remains
an evidence-backed inference and requires confirmation on the macOS CI runner.

## Acceptance Criteria

1. `level1_cursor.rs` contains no fixed sleep between spawning the probe and
   writing the manufactured CPR reply; the reply is written in response to
   observing `ESC[6n` in the master stream.
2. `OscAnswer` is renamed to a protocol-neutral type, and the helper's docs and
   all existing call sites describe both OSC and CPR use accurately.
3. Each cursor test drains through its probe result marker, demonstrating that
   the child completed; merely observing `ESC[6n` is not sufficient.
4. The query-to-reply delay is independent of `Session::spawn` latency. No
   acceptance claim depends on an exact 90ms margin or assumes prompt thread
   wake-up.
5. The three `level1_cursor` tests pass, and the full `biscuit-terminal` suite
   stays green; test counts are reported diagnostically rather than hard-coded.
6. The cursor tests remain non-vacuous: substituting a non-CPR reply makes
   `cursor_position_parses_csi_r_reply` fail with `cursor_position=None` —
   the exact CI symptom — rather than passing.
7. `.config/nextest.toml` parses under both `profile.default` and `profile.ci`.
   On a macOS host, the new filter resolves to the library unit-test binary only
   and yields 1s, existing `browser_` matches retain 5s through override
   precedence, integration binaries retain 100ms, and Linux/Windows retain
   100ms.
8. `biscuit-terminal / test (macos-latest)` reports neither failure. Judge by
   diffing the failure set against the `main` baseline, not by "is it green" —
   the repository carries pre-existing red jobs.

## Non-goals

- **Rewriting `level1_osc_queries.rs`.** Measured to have >160ms of margin in
  the current implementation. Its fixed sleeps remain known timing debt, but
  migrating them is not required to resolve the observed failures.
- **Changing `cursor_position()`'s 100ms default.** The library timeout is a
  product decision about how long to make a user wait on an unresponsive
  terminal. The test must fit the contract, not the reverse.
- **Capping `biscuit-terminal` lib concurrency via a `test-group`.** The
  `tts-audio` and `claudine-windows-l1` groups exist because a shared physical
  resource is contended. Here the cause is generic runner slowness across 2942
  tests; capping concurrency would cost real CI time to solve a 1s-grace
  problem.
- **The WSL2 binary-path failures.** Addressed separately by PR #36.

## Relationship to PR #36

PR #36 (`fix/shared-resolver-for-test-binary`) resolves spawned binary *paths*
at run time for the WSL2 archive leg. There is no functional overlap: these are
macOS timing and teardown defects, which #36 lists as out of scope.

Two points of contact:

- **`.config/nextest.toml` is edited by both.** #36 inserts `archive.include`
  into `[profile.default]` near line 46; this fix appends override blocks near
  lines 171 and 340. Different regions, but the same file — sequence the merges.
- **`bin_exe!` does not apply here.** `discovery_probe` is an *example*, not a
  bin target, so neither `CARGO_BIN_EXE_` nor `NEXTEST_BIN_EXE_` is set for it.
  That is precisely why #36 handled biscuit-terminal via `archive.include`
  instead. `discovery_probe_path()` in `common/pty.rs` derives its path from
  `current_exe()` and remains correct; this fix does not touch it.
