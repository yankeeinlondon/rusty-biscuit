---
source_files_during_phase_2:
  - claudine/cli/src/commands/wrap/exec.rs
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/composition.rs
source_files_during_phase_4:
  - claudine/cli/tests/wrap_commands.rs
  - claudine/cli/src/commands/wrap/composition.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/composition.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/wire_io.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_5:
  - claudine/docs/getting-started/index.md
  - claudine/cli/README.md
  - claudine/docs/cli/sequence.md
  - claudine/docs/topics/non-interactive-sessions.md
  - .claude/skills/claudine/cli-reference.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/cli-reference.md
packages:
  - claudine
---

# Implementation Plan: OpenCode Stability Review Fixes

This plan addresses all findings from `review-1.md` for the OpenCode Stability feature. The review identified four issues: duplicate timeout enforcement in the wait loop, missing timeout config on non-harness compose paths, insufficient test coverage for user-visible diagnostics, and inconsistent `--timeout` CLI grammar.

## Plan Summary

| Phase | Focus | Key Files |
|---|---|---|
| **Phase 1** | Fix `--timeout` duration grammar | `mod.rs`, `composition.rs`, `wire_io.rs` |
| **Phase 2** | Remove duplicate timeout enforcement from wait loop | `exec.rs` |
| **Phase 3** | Thread `TimeoutConfig` into non-harness paths | `composition.rs` |
| **Phase 4** | Add comprehensive tests (Level 1 + Level 2) | `wrap_commands.rs`, new `watchdog_pty.rs` |
| **Phase 5** | Cleanup, lint, typecheck, verify | All modified files |

---

## Phase 1: Fix `--timeout` Duration Grammar

**Goal**: Make `--timeout` accept the same duration grammar as `--step-timeout` (`30s`, `5m`, `2h`, etc.) and reject bare seconds, aligning CLI, frontmatter, env vars, and docs to one grammar.

### 1.1 Change CLI arg type

**File**: `claudine/cli/src/commands/wrap/mod.rs`

At line 713, change:

```rust
#[arg(short = 't', long = "timeout", value_name = "SECONDS")]
pub timeout: Option<u64>,
```

To:

```rust
#[arg(short = 't', long = "timeout", value_name = "DURATION")]
pub timeout: Option<String>,
```

### 1.2 Parse `--timeout` early in the command handler

**File**: `claudine/cli/src/commands/wrap/mod.rs`

Find the block where `cli_step_timeout_secs` is parsed (around line 1028). Add equivalent parsing for `args.timeout`:

```rust
let cli_timeout_duration: Option<std::time::Duration> = match args.timeout.as_deref() {
    Some(raw) => match claudine::harness::parse_timeout(raw, std::path::Path::new("<cli>")) {
        Ok(d) => Some(d),
        Err(e) => {
            return Err(eyre!("--timeout: {e}"));
        }
    },
    None => None,
};
```

Zero durations from the CLI must be rejected (the spec says disable by omission or env var, not CLI):

```rust
if cli_timeout_duration == Some(std::time::Duration::from_secs(0)) {
    return Err(eyre!("--timeout: zero duration is not accepted; omit the flag to disable the wall-clock timeout"));
}
```

### 1.3 Update `resolve_timeouts` signature and implementation

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Change `resolve_timeouts` to accept `Option<String>` for CLI values instead of `Option<u64>`, and parse inside the function:

```rust
pub(crate) fn resolve_timeouts(
    cli_timeout: Option<String>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    plan_step_timeout: Option<std::time::Duration>,
) -> super::subagent_watchdog::TimeoutConfig {
```

Update `TimeoutResolutionInput`:

```rust
pub(crate) struct TimeoutResolutionInput<'a> {
    pub cli: Option<String>,
    pub frontmatter: Option<std::time::Duration>,
    pub env_var: &'a str,
    pub built_in: Option<std::time::Duration>,
}
```

Update `resolve_single_timeout` to parse the CLI string:

```rust
pub(crate) fn resolve_single_timeout(
    input: TimeoutResolutionInput<'_>,
) -> Option<std::time::Duration> {
    if let Some(raw) = input.cli {
        match claudine::harness::parse_timeout(&raw, std::path::Path::new("<cli>")) {
            Ok(d) => return Some(d),
            Err(_) => {
                // Invalid CLI value should have been caught earlier, but
                // fall through rather than panicking.
            }
        }
    }
    // ... rest unchanged
}
```

### 1.4 Update all call sites of `resolve_timeouts`

**File**: `claudine/cli/src/commands/wrap/mod.rs`

Update all call sites that pass `args.timeout` (currently `Option<u64>`) to pass `args.timeout.clone()` (now `Option<String>`). Key locations:

- Line 1651: `args.timeout` → `args.timeout.clone()`
- Line 1720: `timeout: args.timeout` in `wire_io::run_kimi_wire_session` — this needs special handling (see 1.5)
- Line 1736: `args.timeout` → `args.timeout.clone()`
- Line 1810: `args.timeout` → `args.timeout.clone()`
- Lines 3137, 3157: `cli_timeout` — these already receive the parsed value; update the caller to pass the `Option<String>` instead

Also update `resolve_launch_timeouts` and `build_harness_launch` signatures to accept `Option<String>` for CLI timeout, and parse inside or remove the old `LaunchTimeouts` struct entirely (see Phase 2).

### 1.5 Update Kimi wire session timeout

**File**: `claudine/cli/src/commands/wrap/wire_io.rs`

The `WireSessionConfig.timeout` is currently `Option<u64>` (seconds). Change it to `Option<std::time::Duration>` and update the consumer at line 779:

```rust
let exit_code = match wait_for_child_exit(
    &mut child,
    config.timeout,  // already Option<Duration>
    ...
) {
```

Update `wait_for_child_exit` signature accordingly.

### 1.6 Update existing unit tests

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Update the `resolve_timeouts` unit tests (lines 2266–2342) to pass `Option<String>` for CLI values:

```rust
let cfg = resolve_timeouts(
    Some("60s".into()),
    Some(std::time::Duration::from_secs(120)),
    Some("45s".into()),
    Some(std::time::Duration::from_secs(90)),
);
```

Add new tests:
- `resolve_timeouts_rejects_bare_seconds_cli` — passing `Some("60".into())` should behave as an invalid parse (fall through to next precedence layer). Actually, `parse_timeout` rejects bare seconds, so this should fall through.
- `resolve_timeouts_accepts_hour_and_minute_cli` — `Some("2h".into())`, `Some("30m".into())`.

### 1.7 Update integration tests

**File**: `claudine/cli/tests/wrap_commands.rs`

Find tests that use `--timeout 30` (lines 1339, 1346, 1351) and update to `--timeout 30s`. Also update the test assertion that checks for `"--timeout can only be used in non-interactive mode"` — the value can stay as `30s`.

---

## Phase 2: Remove Duplicate Timeout Enforcement from Wait Loop

**Goal**: The `wait_with_signal_and_early_termination` function must only consume termination requests from the watchdog ticker channel (and the existing stderr-bridge early-termination channel). It must not independently evaluate `timeout` or `step_timeout`. This prevents the 75 ms poll loop from racing with the 5 s ticker and bypassing the rendered `Agent Error` block.

### 2.1 Remove direct wall-clock timeout branch

**File**: `claudine/cli/src/commands/wrap/exec.rs`

Delete lines 819–842 (the `if !wall_clock_tripped && early_termination.is_none() && let Some(budget) = wall_clock_timeout ...` block).

### 2.2 Remove direct step-timeout branch

**File**: `claudine/cli/src/commands/wrap/exec.rs`

Delete the `detect_step_timeout` branch (lines ~897–919). The exact block looks like:

```rust
if early_termination.is_none()
    && !wall_clock_tripped
    && let Some(metrics) = live_metrics.as_ref()
{
    // ... detect_step_timeout logic ...
}
```

Remove this entire block.

### 2.3 Update wait-loop signature

**File**: `claudine/cli/src/commands/wrap/exec.rs`

Remove `wall_clock_timeout: Option<Duration>` and `step_timeout: Option<Duration>` from the `wait_with_signal_and_early_termination` signature (lines 752–753).

Update the doc comment (lines 730–742) to remove references to "wall-clock" and "step" timeout enforcement inside this function; instead document that the function consumes `EarlyTermination` and `WatchdogTermination` signals from channels.

### 2.4 Update wait-loop call site

**File**: `claudine/cli/src/commands/wrap/exec.rs`

At lines 2289–2299, remove the two timeout arguments from the call:

```rust
wait_with_signal_and_early_termination(
    &mut child,
    true,
    rx,
    wd_rx,
    Some(wait_loop_metrics),
    opencode_stop_threshold,
    // REMOVED: wall_clock_timeout,
    // REMOVED: step_timeout_duration,
    timeout_config.kill_grace,
)?
```

### 2.5 Update `needs_advanced_wait` logic

**File**: `claudine/cli/src/commands/wrap/exec.rs`

At lines 2272–2275, simplify:

```rust
let needs_advanced_wait = early_terminate_rx.is_some()
    || watchdog_enabled;
```

The timeout rules no longer need to trigger the advanced wait path directly; the watchdog channel is sufficient.

### 2.6 Remove `wall_clock_tripped` from termination classification

**File**: `claudine/cli/src/commands/wrap/exec.rs`

In the `try_wait` success branch (lines 801–816), the termination classification currently checks `wall_clock_tripped`. Since the ticker now sends `WatchdogTermination` with reason `Timeout`, which is converted to `EarlyTermination::Timeout`, the `early_termination.is_some()` branch already handles this. Remove the `else if wall_clock_tripped` branch:

```rust
let termination = if was_interrupted {
    claudine::harness::ProcessTermination::Interrupted
} else if early_termination.is_some() {
    early_termination_process_outcome(early_termination.as_ref())
} else {
    claudine::harness::ProcessTermination::Completed
};
```

Also remove the `wall_clock_tripped` mutable variable (line 794).

### 2.7 Remove legacy `LaunchTimeouts` and old fields from `AttemptLaunch`

**File**: `claudine/cli/src/commands/wrap/mod.rs`

The review explicitly recommends removing the old fields: "remove the older timeout fields from `AttemptLaunch` / wait-loop plumbing once `TimeoutConfig` is the single contract."

1. Remove `LaunchTimeouts` struct (lines 2046–2059).
2. Remove `resolve_launch_timeouts` function (lines 2062–2073).
3. Remove `timeout: Option<u64>` and `step_timeout: Option<u64>` from `AttemptLaunch` (lines 140, 146).
4. Update `build_harness_launch` to only populate `timeout_config` (lines 2110–2131).
5. Update all consumers of `launch.timeout` and `launch.step_timeout`:
   - Line 2174: `launch.step_timeout.is_some()` → `launch.timeout_config.step_timeout.is_some()`
   - Line 2186: `adjusted.step_timeout = None` → remove (no longer exists)
   - Line 2229: `timeout: launch.timeout` for Kimi wire — change `WireSessionConfig.timeout` to `Option<Duration>` and pass `launch.timeout_config.timeout`
   - Lines 3135–3145: `resolved_timeouts` for span — compute from `launch.timeout_config` directly
   - Any span/logging that reads `timeout_secs` / `step_timeout_secs`

### 2.8 Update `run_harness_loop` signature

**File**: `claudine/cli/src/commands/wrap/mod.rs`

Remove `cli_timeout: Option<u64>` and `cli_step_timeout: Option<u64>` parameters from `run_harness_loop` (lines 2841–2842). The harness loop should receive `TimeoutConfig` directly inside the `AttemptLaunch` struct, which it already does.

Update the call site at line 2313: remove the `launch.timeout` argument.

---

## Phase 3: Thread `TimeoutConfig` into Non-Harness Compose/Inline-Compose

**Goal**: Ensure `compose --timeout 2h --step-timeout 5m prompt.md` (without harness frontmatter) actually applies the CLI values.

### 3.1 Parse CLI timeout values before calling `execute_without_harness`

**File**: `claudine/cli/src/commands/wrap/composition.rs`

At the call site (line 1264), resolve `TimeoutConfig` from the request fields and pass it in:

```rust
let timeout_config = resolve_timeouts(
    request.timeout.clone(),   // Option<String>
    None,                      // no frontmatter timeout on non-harness path
    request.step_timeout.clone(), // Option<String>
    None,                      // no frontmatter step_timeout on non-harness path
);

let exit_result = execute_without_harness(
    mode,
    provider,
    profile,
    &binary_path,
    &child_args,
    &env_plan.env,
    child_cwd,
    stdin_seed.as_deref(),
    wire_prompt.as_deref(),
    use_structured,
    structured_codex_output.as_ref(),
    stdout_noise,
    stderr_noise,
    stream_verbosity,
    detail_requested,
    &env_context,
    &dispatch_context,
    &term,
    &mut child_spawned,
    prompt_timing,
    &mut agent_perf,
    timeout_config,  // NEW
);
```

### 3.2 Update `execute_without_harness` signature

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Add `timeout_config: super::subagent_watchdog::TimeoutConfig` parameter.

### 3.3 Thread timeout into `run_structured_composition`

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Inside `execute_without_harness`, the `run_structured_composition` call (around line 1368) must pass `timeout_config` through.

Update `run_structured_composition` signature to accept `timeout_config: super::subagent_watchdog::TimeoutConfig`.

At line 1784, replace:

```rust
let timeout_config = resolve_timeouts(None, None, None, None);
```

With:

```rust
// Use the already-resolved timeout_config passed from the caller.
```

(Just use the parameter directly.)

### 3.4 Thread timeout into legacy (non-structured) path

**File**: `claudine/cli/src/commands/wrap/composition.rs`

The legacy path inside `execute_without_harness` (lines 1414–1479) currently calls `exec::run_child` and `exec::run_child_capture`, which do not support timeout enforcement. For parity:

- If `timeout_config.any_enabled()` and the path is non-structured, emit a warning to stderr (once, not per-attempt) that timeouts are only enforced in structured-stream mode, then proceed without timeout. This matches the existing harness behavior (line 2174–2188 in mod.rs).

Alternatively, since `run_child` and `run_child_capture` do not have the structured stream plumbing needed for the watchdog, this is acceptable — but the warning ensures the user knows.

### 3.5 Add CLI-precedence tests for non-harness compose

**File**: `claudine/cli/tests/wrap_commands.rs`

Add two new tests:

```rust
/// Non-harness compose respects --timeout CLI flag (duration grammar).
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_non_harness_respects_cli_timeout() {
    // ... setup fake opencode that emits events forever ...
    // Run: claudine compose --opencode --timeout 2s prompt.md
    // Assert: run terminates within ~5s with "wall-clock" in stderr.
}

/// Non-harness inline-compose respects --step-timeout CLI flag.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn inline_compose_non_harness_respects_cli_step_timeout() {
    // ... setup fake opencode that emits one event then blocks ...
    // Run: claudine inline-compose --opencode --step-timeout 2s prompt.md
    // Assert: run terminates with "step_timeout" in stderr.
}
```

---

## Phase 4: Add Comprehensive Tests

### 4.1 Level 1: Exact stderr text shape and JSONL summary fields

**File**: `claudine/cli/tests/wrap_commands.rs`

Enhance the existing watchdog tests (lines 4861–5049) to assert exact shapes instead of substring containment:

#### 4.1.1 `watchdog_subagent_hang_terminates_and_names_stuck_ids`

Current assertions:
- `plain.contains("step_timeout")`
- `plain.contains("sa8") || plain.contains("Task 8")`

Replace with exact assertions:

```rust
// The rendered error block must contain the exact stuck-subagent enumeration.
assert!(
    plain.contains("2 subagents were still outstanding"),
    "stderr should enumerate stuck subagent count; got: {plain}"
);
assert!(
    plain.contains("sa8 \"Task 8\""),
    "stderr should name stuck subagent 8 with id and name; got: {plain}"
);
assert!(
    plain.contains("sa9 \"Task 9\""),
    "stderr should name stuck subagent 9 with id and name; got: {plain}"
);
```

Also assert the JSONL summary field:

```rust
let log_path = today_log_path(workspace.path());
if log_path.exists() {
    let log = fs::read_to_string(&log_path).unwrap();
    let last = log.lines().last().unwrap();
    let entry: serde_json::Value = serde_json::from_str(last).unwrap();
    assert_eq!(
        entry.get("exit_reason").and_then(|v| v.as_str()),
        Some("step_timeout"),
        "JSONL session_end must have exit_reason=step_timeout; last entry: {last}"
    );
}
```

#### 4.1.2 `watchdog_wall_clock_timeout_terminates_active_stream`

Add JSONL assertion for `exit_reason: "timeout"`.

#### 4.1.3 Add test for `Awaiting subagent` diagnostic line

```rust
/// Before step_timeout kills the run, the flush ticker emits ⏳ Awaiting
/// subagent lines for each outstanding subagent.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn watchdog_emits_awaiting_subagent_diagnostic_before_kill() {
    // Fake opencode: emit 3 task_started, 1 task_completed, then silence.
    // Configure low step_timeout (3s) and low SILENCE_WINDOW (1s) so the
    // flush ticker fires quickly.
    // Assert stderr contains "Awaiting subagent" with elapsed time.
}
```

**Note**: The `SILENCE_WINDOW` is currently hardcoded at 30s in `exec.rs`. For this test to run fast, we need either:
- A test-only env var override for `SILENCE_WINDOW`, or
- Make `SILENCE_WINDOW` configurable through `TimeoutConfig` or an env var.

Recommended: add `CLAUDINE_FLUSH_SILENCE_WINDOW` env var read in `exec.rs` (with default 30s) so tests can lower it. This is a small, safe addition.

### 4.2 Level 2: Terminal rendering tests

**New File**: `claudine/cli/tests/watchdog_pty.rs`

Create a new PTY test file modeled after `validation_reporter_pty.rs`. These tests verify:
- `Agent Error` block border rendering (red border, correct glyph)
- `Awaiting subagent` line rendering with elapsed time
- OSC-8 hyperlinks and SGR styling in the error block

Approach: Use a dedicated test-only binary (like `validation_reporter_pty_harness`) that calls `render_watchdog_error_to_stream` and `diagnostic_lines` through a public API, or drive the full `compose` pipeline under PTY with a fake provider.

Given the complexity of staging a full `compose` pipeline under PTY, the plan recommends a hybrid:

1. **Test-only binary**: `claudine/cli/src/bin/watchdog_pty_harness.rs`
   - Calls `subagent_watchdog::render_watchdog_error_to_stream` with a synthetic `WatchdogTermination`
   - Also renders `Awaiting subagent` diagnostic lines via the same `StreamOutput` path
   - Writes to stderr and exits

2. **PTY test**: `claudine/cli/tests/watchdog_pty.rs`
   - Spawns the harness binary under `expectrl::Session`
   - Captures the full ANSI transcript
   - Asserts:
     - `\x1b[31m` (red SGR) present in the error block
     - `Agent Error` text present
     - Block border characters present (e.g., `▌` or box-drawing chars)
     - `Awaiting subagent` text present with elapsed duration pattern

**File**: `claudine/cli/src/bin/watchdog_pty_harness.rs`

```rust
fn main() {
    let stream_output = /* construct StreamOutput */;
    let termination = WatchdogTermination {
        reason: WatchdogTerminationReason::StepTimeout,
        message: "no stream activity for 2s. ...".into(),
        stuck_subagents: vec![
            ActiveSubagentSnapshot { id: "sa1".into(), name: Some("Task 1".into()), ... },
        ],
    };
    render_watchdog_error_to_stream(&termination, &stream_output);

    // Also emit diagnostic lines
    let mut state = WatchdogState::default();
    state.subagent_started_now("sa1".into(), Some("Task 1".into()));
    let lines = state.diagnostic_lines(Instant::now(), Duration::from_secs(1));
    for line in lines {
        stream_output.emit_stderr_line(&format!(
            "Awaiting subagent: {} ({})",
            line.display_name,
            format_duration(line.elapsed_since_start)
        ));
    }
}
```

**File**: `claudine/cli/tests/watchdog_pty.rs`

```rust
#[test]
#[ignore = "PTY tests are timing-sensitive; run locally with --ignored"]
fn pty_watchdog_error_block_renders_red_border_and_agent_error_header() {
    let transcript = run_harness_under_pty();
    assert!(transcript.contains("Agent Error"));
    assert!(transcript.contains("\x1b[31m")); // red SGR
    assert!(transcript.contains("▌")); // block border
}

#[test]
#[ignore = "PTY tests are timing-sensitive; run locally with --ignored"]
fn pty_watchdog_awaiting_subagent_line_renders_with_elapsed() {
    let transcript = run_harness_under_pty();
    assert!(transcript.contains("Awaiting subagent"));
    assert!(transcript.contains("Task 1"));
    // Elapsed pattern like "(1s)" or "(0s)"
    assert!(transcript.contains('(') && transcript.contains(')'));
}
```

### 4.3 Unit tests for `TimeoutConfig::resolve` precedence

**File**: `claudine/cli/src/commands/wrap/subagent_watchdog.rs`

Add tests (already partially present; enhance):

```rust
#[test]
#[serial_test::serial]
fn timeout_config_resolve_cli_wins_over_frontmatter_env_and_default() {
    let _g1 = TestEnvGuard::clear("CLAUDINE_TIMEOUT");
    let _g2 = TestEnvGuard::clear("CLAUDINE_STEP_TIMEOUT");
    let _g3 = TestEnvGuard::clear("CLAUDINE_KILL_GRACE");
    let _g4 = TestEnvGuard::clear("CLAUDINE_WATCHDOG_INTERVAL");

    // Simulating the composition layer resolving CLI > frontmatter > env
    let resolved_timeout = Some(Duration::from_secs(7200)); // from CLI
    let resolved_step_timeout = Some(Duration::from_secs(1800)); // from CLI
    let config = TimeoutConfig::resolve(resolved_timeout, resolved_step_timeout);
    assert_eq!(config.timeout, Some(Duration::from_secs(7200)));
    assert_eq!(config.step_timeout, Some(Duration::from_secs(1800)));
}
```

Also add tests in `composition.rs`:

```rust
#[test]
#[serial_test::serial]
fn resolve_timeouts_cli_duration_string_parsed() {
    let cfg = resolve_timeouts(
        Some("2h".into()),
        None,
        Some("5m".into()),
        None,
    );
    assert_eq!(cfg.timeout, Some(Duration::from_secs(7200)));
    assert_eq!(cfg.step_timeout, Some(Duration::from_secs(300)));
}

#[test]
#[serial_test::serial]
fn resolve_timeouts_cli_zero_rejected() {
    // Zero from CLI should be rejected early, but if it reaches resolve_timeouts,
    // parse_timeout rejects it and falls through.
    let cfg = resolve_timeouts(
        Some("0s".into()),
        None,
        Some("0s".into()),
        None,
    );
    // parse_timeout rejects 0s, so CLI layer falls through to next precedence
    // (which is None here), resulting in built-in defaults.
    assert_eq!(cfg.timeout, None);
    assert_eq!(cfg.step_timeout, Some(Duration::from_secs(30 * 60)));
}
```

---

## Phase 5: Cleanup, Lint, Typecheck, and Verify

### 5.1 Run tests

```bash
# Unit tests for modified modules
cargo test -p claudine-cli subagent_watchdog
cargo test -p claudine-cli composition::tests
cargo test -p claudine-cli exec

# Integration tests
cargo test -p claudine-cli --test wrap_commands watchdog
cargo test -p claudine-cli --test wrap_commands timeout

# New PTY tests (ignored by default)
cargo test -p claudine-cli --test watchdog_pty -- --ignored

# Full CLI test suite
cargo test -p claudine-cli --test wrap_commands
```

### 5.2 Run lint and typecheck

```bash
# If a justfile exists for claudine:
just lint
just test

# Otherwise:
cargo clippy -p claudine-cli --all-targets -- -D warnings
cargo check -p claudine-cli --all-targets
cargo fmt -- --check
```

### 5.3 Verify no dead code warnings

The `subagent_watchdog.rs` file starts with `#![allow(dead_code)]`. After Phase 2, much of this code is actively used. Remove the `#![allow(dead_code)]` pragma and fix any legitimate warnings.

### 5.4 Update README/docs if needed

If the README documents `--timeout <SECONDS>`, update it to `--timeout <DURATION>` with examples (`2h`, `30m`, `10s`).

---

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Removing old `AttemptLaunch.timeout` breaks Kimi wire session | Update `WireSessionConfig.timeout` to `Option<Duration>` and test with a unit test |
| `parse_timeout` rejects `0s` but env-var disable relies on `is_zero_duration_literal` | Ensure the zero-disable path still works: `is_zero_duration_literal` is checked **before** `parse_timeout` in `resolve_single_timeout`, so env vars still bypass correctly |
| PTY tests are flaky on CI | Mark them `#[ignore]` and run only locally; the Level 1 tests provide deterministic coverage |
| Non-structured path (capture/passthrough) no longer has timeout warning | Add the warning in `execute_without_harness` for the legacy branch |
| Large refactor touches many call sites | Make changes incrementally per phase; compile after each phase |

## Acceptance Criteria

- [ ] `cargo test -p claudine-cli --test wrap_commands` passes
- [ ] `cargo clippy -p claudine-cli --all-targets -- -D warnings` passes with no errors
- [ ] `--timeout 2h` and `--step-timeout 5m` work on `compose` without harness frontmatter
- [ ] The wait loop no longer contains direct `wall_clock_timeout` or `step_timeout` enforcement
- [ ] Watchdog ticker is the sole source of timeout-driven `EarlyTermination`
- [ ] JSONL `session_end` entries contain `exit_reason: "timeout"` or `"step_timeout"` as appropriate
- [ ] Level 1 tests assert exact stuck-subagent id/name enumeration
- [ ] Level 2 PTY tests verify `Agent Error` block and `Awaiting subagent` rendering
- [ ] All existing tests continue to pass
