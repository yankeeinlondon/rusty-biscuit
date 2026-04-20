# Plan 1: Hook Handler Hang

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop `claudine handle <event>` from blocking for 30 s (until the parent agent's hook-timeout machinery kills it) when fired by Claude / Gemini / Codex session hooks. The hang is the root cause of every wrap session feeling "hung"; fixing it unblocks every subsequent plan.

**Architecture:** Add an unconditional per-invocation execution deadline enforced inside `claudine handle ...`, structured phase-level tracing so any future hang produces actionable logs, and targeted instrumentation around the dispatch action types most likely to block (messaging, bash, TTS, sound, log-writers). The deadline is hard-coded default plus an env-var override; no new CLI flag. Hook handlers that can't complete in the deadline print a one-line diagnostic to stderr and exit non-zero so the agent's hook machinery classifies them as "failed" rather than "timed out" — giving the user clear signal the handler misbehaved instead of the ambiguous 30 s wait.

**Tech Stack:** Rust, tokio, tracing, claudine CLI (`claudine/cli/src/commands/handle.rs`), claudine dispatch (`claudine/lib/src/dispatch/mod.rs`).

**Evidence trail (from `agent-output/gemini.err` captured 2026-04-14):**
```
Hook execution for AfterAgent: 0 succeeded, 1 failed (claudine-turn_complete), total duration: 30004ms
Hook execution error: Hook timed out after 30000ms
Hook execution for SessionEnd: 0 succeeded, 1 failed (claudine-session_end), total duration: 30004ms
```
Claude's `agent-output/claude.err` shows the same symptom through a different path (`SessionEnd hook [claudine handle session_end] failed: Hook cancelled`). Codex's `agent-output/codex.err` shows a tool-stdin hang that is orthogonal to this plan.

---

## File Map

| File | Change | Purpose |
|------|--------|---------|
| `claudine/cli/src/commands/handle.rs` | Modify | Wrap the `run` body in `tokio::time::timeout`; add phase-level `tracing::info_span!`s; emit a one-line diagnostic on timeout before exiting non-zero |
| `claudine/lib/src/dispatch/mod.rs` | Modify | Add phase spans (`stdin_read`, `provider_resolve`, `dispatch_canonical`, `action_*`) so a future hang log identifies the culprit phase |
| `claudine/lib/src/actions/bash_executor.rs` | Modify | Default per-bash-action timeout of 5 s inside hook handlers (currently unbounded); surface timeout as a `DispatchOutcome` error, not a hang |
| `claudine/lib/src/messaging/send.rs` | Modify | Default per-message timeout of 5 s on outbound Discord/Slack/Signal/WhatsApp sends when running inside `claudine handle` |
| `claudine/cli/tests/handle_commands.rs` | Modify | Add integration tests: deadline fires, bash-timeout surfaces as error, stdin-EOF read succeeds, fast path unaffected |

---

## Preconditions

- [ ] **Step 0: Confirm working tree is clean on `claudine` branch**

Run: `git status --short`
Expected: no modifications under `claudine/cli/src/commands/handle.rs`, `claudine/lib/src/dispatch/`, `claudine/lib/src/actions/`, or `claudine/lib/src/messaging/`.

If dirty in those paths, surface to the user before continuing.

- [ ] **Step 0.1: Capture baseline hang reproduction**

Record that this plan starts from the captured fixtures at `claudine/claudine-output/*.err` showing the 30 s hang on `gemini`, and that after Plan 1 ships, a re-run of the same prompt must complete in under (default deadline) seconds.

---

## Task 1: Introduce a hard execution deadline in `claudine handle`

**Files:**
- Modify: `claudine/cli/src/commands/handle.rs`

- [ ] **Step 1: Write the failing integration test**

Append to `claudine/cli/tests/handle_commands.rs` (create the file if it doesn't exist — follow the pattern of `claudine/cli/tests/wrap_commands.rs`):

```rust
use std::process::Stdio;
use std::time::{Duration, Instant};

#[test]
fn handle_aborts_when_deadline_exceeded() {
    // Event payload designed so dispatch would fire a bash action that sleeps
    // for 60s. With the deadline enforced (default 5s), the command must exit
    // in under 8s with exit code non-zero and a stderr line that names the
    // phase that was still running.
    let payload = serde_json::json!({
        "hook_event_name": "SessionEnd",
        "provider": "claude",
        "session_id": "deadline-test",
        "claudine_test_force_sleep_seconds": 60
    })
    .to_string();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_claudine"))
        .arg("handle")
        .arg("session_end")
        .arg("--provider")
        .arg("claude")
        .env("CLAUDINE_HANDLE_DEADLINE_SECONDS", "5")
        .env("CLAUDINE_TEST_FORCE_SLEEP", "60")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    drop(child.stdin.take());

    let start = Instant::now();
    let output = child.wait_with_output().expect("wait");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(8),
        "handle should exit within ~5s deadline + grace; took {elapsed:?}"
    );
    assert!(!output.status.success(), "expected non-zero exit on timeout");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("deadline exceeded"),
        "expected 'deadline exceeded' in stderr: {stderr}"
    );
}
```

The test uses two environment variables (`CLAUDINE_HANDLE_DEADLINE_SECONDS` and `CLAUDINE_TEST_FORCE_SLEEP`) that Task 1 and Task 3 will add.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p claudine-cli --test handle_commands handle_aborts_when_deadline_exceeded`
Expected: FAIL — either the test hangs (no deadline yet) or the assertion fires (no env-var recognized). If it hangs past 8 seconds, kill it; that confirms the bug.

- [ ] **Step 3: Wrap the `run` body in `tokio::time::timeout`**

In `claudine/cli/src/commands/handle.rs`, modify the `run` function:

```rust
/// Handle an incoming event from stdin.
pub async fn run(args: HandleArgs) -> Result<()> {
    let deadline_secs = std::env::var("CLAUDINE_HANDLE_DEADLINE_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_HANDLE_DEADLINE_SECONDS);

    let deadline = std::time::Duration::from_secs(deadline_secs);

    match tokio::time::timeout(deadline, run_inner(args)).await {
        Ok(result) => result,
        Err(_elapsed) => {
            eprintln!(
                "claudine handle: deadline exceeded after {deadline_secs}s; \
                 aborting hook handler to prevent blocking the agent session"
            );
            std::process::exit(EXIT_CODE_DEADLINE_EXCEEDED);
        }
    }
}

const DEFAULT_HANDLE_DEADLINE_SECONDS: u64 = 5;
const EXIT_CODE_DEADLINE_EXCEEDED: i32 = 124;

async fn run_inner(args: HandleArgs) -> Result<()> {
    let raw = read_stdin_json()?;
    let provider = resolve_provider(args.provider, &raw)?;
    let cwd = std::env::current_dir().unwrap_or_default();
    let env = detect_environment_fast(&cwd);

    let event_label = args.event.as_deref().unwrap_or("event");
    debug!(%provider, event = %event_label, "Handling event");
    let outcome = claudine::dispatch::dispatch_canonical(&raw, provider, &env).await?;
    if args.json {
        let output = serde_json::json!({
            "provider": provider.as_slug(),
            "event": event_label,
            "response": outcome.response,
            "exit_code": outcome.exit_code,
            "protect_pre": outcome.protect_pre,
            "protect_post": outcome.protect_post,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if let Some(payload) = outcome.response {
        println!("{}", serde_json::to_string(&payload)?);
    }
    if let Some(exit_code) = outcome.exit_code {
        std::process::exit(exit_code);
    }
    Ok(())
}
```

Exit code `124` matches the `coreutils timeout` convention, signalling "operation timed out" to any shell or agent that inspects exit codes.

- [ ] **Step 4: Confirm the test still fails (dispatch has no 60 s hook yet to trigger)**

Run the test again. It will still fail because no 60 s hook exists to time out against. That's fine — Task 3 introduces the test hook.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/handle.rs claudine/cli/tests/handle_commands.rs
git commit -m "feat(claudine): enforce execution deadline in claudine handle

Adds a 5s default deadline (overridable via
CLAUDINE_HANDLE_DEADLINE_SECONDS) around the claudine handle subcommand
body. When exceeded, prints 'claudine handle: deadline exceeded after
{N}s' to stderr and exits 124 (coreutils 'operation timed out'
convention). This stops hook handlers from blocking the parent agent
session — previously they would hang for ~30s until the agent's own
hook timeout machinery killed them, producing the 'claudine hung on
every provider' symptom observed in the 2026-04-14 capture."
```

Scope: use `git add` with the explicit paths. No `git add -A`. No `Co-Authored-By` trailer.

---

## Task 2: Add phase-level tracing spans so the next hang is debuggable

**Files:**
- Modify: `claudine/cli/src/commands/handle.rs`
- Modify: `claudine/lib/src/dispatch/mod.rs`

- [ ] **Step 1: Add tracing import and instrument `run_inner`**

In `claudine/cli/src/commands/handle.rs`, wrap each phase in an `info_span`:

```rust
async fn run_inner(args: HandleArgs) -> Result<()> {
    let raw = {
        let _span = tracing::info_span!("handle_stdin_read").entered();
        read_stdin_json()?
    };
    let provider = {
        let _span = tracing::info_span!("handle_provider_resolve").entered();
        resolve_provider(args.provider, &raw)?
    };
    let (cwd, env) = {
        let _span = tracing::info_span!("handle_env_detect").entered();
        let cwd = std::env::current_dir().unwrap_or_default();
        let env = detect_environment_fast(&cwd);
        (cwd, env)
    };
    let _ = cwd; // not used further after env derivation

    let event_label = args.event.as_deref().unwrap_or("event");
    debug!(%provider, event = %event_label, "Handling event");

    let outcome = {
        let span = tracing::info_span!(
            "handle_dispatch_canonical",
            %provider,
            event = %event_label
        );
        let _enter = span.enter();
        claudine::dispatch::dispatch_canonical(&raw, provider, &env).await?
    };

    // ... (remainder unchanged)
}
```

- [ ] **Step 2: Instrument the dispatch pipeline**

In `claudine/lib/src/dispatch/mod.rs`, find `dispatch_canonical` (grep for `pub async fn dispatch_canonical`). Wrap the main body in nested spans for the major phases:

- `load_config`
- `resolve_bindings`
- `template_interpolate`
- `run_pre_actions` (protect_pre)
- `run_bindings` (one span per binding type: bash / messenger / speak / sound / log / report)
- `run_post_actions` (protect_post)

Use `tracing::info_span!("phase_name", %provider, event = ?event)` and enter them with `.enter()` or `.in_scope(async move { ... })` depending on the existing control flow. Preserve every existing tracing call; add spans, don't remove.

- [ ] **Step 3: Verify spans are observable under `RUST_LOG=claudine=debug`**

Run (in a temporary test shell):

```bash
RUST_LOG=claudine=debug cargo run -p claudine-cli -- handle session_end --provider claude <<'EOF'
{"hook_event_name": "SessionEnd", "session_id": "debug-test"}
EOF
```

Confirm that the tracing output shows entries for `handle_stdin_read`, `handle_provider_resolve`, `handle_dispatch_canonical`, and at least one of `load_config` / `resolve_bindings` / `run_bindings`.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/handle.rs claudine/lib/src/dispatch/mod.rs
git commit -m "feat(claudine): add phase spans to claudine handle dispatch

Every phase of handle dispatch now emits a structured tracing span
(stdin_read, provider_resolve, env_detect, dispatch_canonical,
load_config, resolve_bindings, template_interpolate, run_pre_actions,
run_bindings, run_post_actions). A future hang surfaces the offending
phase in RUST_LOG=claudine=debug output rather than appearing as an
opaque 30s wait."
```

---

## Task 3: Per-action timeouts on the two blocking-prone action types

**Files:**
- Modify: `claudine/lib/src/actions/bash_executor.rs`
- Modify: `claudine/lib/src/messaging/send.rs`
- Modify: `claudine/cli/tests/handle_commands.rs`

Rationale: the two action types most likely to block indefinitely inside `dispatch_canonical` are `bash` (shell command with no default timeout) and `messaging` (outbound HTTP to Discord / Slack / Signal / WhatsApp, where a dead endpoint sits on a connect deadline). Both must have a hard per-action deadline when invoked inside `claudine handle` so a single stuck action cannot exhaust the whole 5 s handle deadline.

- [ ] **Step 1: Write the failing test for bash-action timeout**

Append to `claudine/cli/tests/handle_commands.rs`:

```rust
#[test]
fn handle_bash_action_times_out_cleanly() {
    // Stage: a bash action that sleeps 60s should be aborted by the per-action
    // timeout (default 5s inside handle) and surface as a non-fatal action
    // failure — the overall dispatch still completes well inside the handle
    // deadline.
    let config_dir = tempfile::tempdir().expect("tempdir");
    let config_path = config_dir.path().join("claudine.toml");
    std::fs::write(
        &config_path,
        r#"
[hooks.session_end]
bash = "sleep 60"
"#,
    )
    .expect("write config");

    let payload = serde_json::json!({
        "hook_event_name": "SessionEnd",
        "provider": "claude",
        "session_id": "bash-timeout-test"
    })
    .to_string();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_claudine"))
        .arg("handle")
        .arg("session_end")
        .arg("--provider")
        .arg("claude")
        .env("CLAUDINE_HANDLE_DEADLINE_SECONDS", "10")
        .env("CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS", "2")
        .env("CLAUDINE_CONFIG_PATH", config_path.to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    drop(child.stdin.take());

    let start = std::time::Instant::now();
    let output = child.wait_with_output().expect("wait");
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "bash action should time out in ~2s; total took {elapsed:?}"
    );
    // Per-action timeout is a soft failure: we expect overall handle to still
    // exit cleanly (exit 0) while stderr names the timed-out action.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("bash action timed out"),
        "expected bash timeout notice in stderr: {stderr}"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p claudine-cli --test handle_commands handle_bash_action_times_out_cleanly`
Expected: FAIL — either the test hangs (no per-action timeout) or the assertion fires (`CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS` isn't honored).

- [ ] **Step 3: Add per-action timeout to `bash_executor.rs`**

In `claudine/lib/src/actions/bash_executor.rs`, locate the function that invokes the shell command (grep for `tokio::process::Command` or `Command::new`). Wrap the `.wait()` / `.status()` await in `tokio::time::timeout`:

```rust
const DEFAULT_BASH_ACTION_TIMEOUT_SECONDS: u64 = 30;

pub async fn execute_bash_action(
    script: &str,
    context: &BashExecutionContext,
) -> Result<BashExecutionOutcome, BashActionError> {
    let timeout_secs = std::env::var("CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_BASH_ACTION_TIMEOUT_SECONDS);

    let deadline = std::time::Duration::from_secs(timeout_secs);

    // existing command construction ...
    let spawn_future = async {
        // existing spawn/wait body
    };

    match tokio::time::timeout(deadline, spawn_future).await {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                script_preview = %crate::actions::bash_executor::preview(script),
                timeout_secs,
                "bash action timed out"
            );
            eprintln!("bash action timed out after {timeout_secs}s");
            Err(BashActionError::Timeout {
                timeout_secs,
                script: script.to_string(),
            })
        }
    }
}
```

Extend `BashActionError` with a `Timeout { timeout_secs: u64, script: String }` variant (follow existing `thiserror::Error` pattern; update the `#[error(...)]` annotation). Update all existing `match`/`.map_err` sites that consume `BashActionError` to handle the new variant (most should propagate or log; none should panic).

Confirm the existing default of 30 s remains for non-handle invocations by keeping the env var opt-in; only the `handle` path sets `CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS` to a tighter value (Task 3.5 below).

- [ ] **Step 3.5: Tighten the default inside `claudine handle`**

In `claudine/cli/src/commands/handle.rs`, `run_inner` (from Task 1), set the bash action timeout env var if not already set by the caller:

```rust
async fn run_inner(args: HandleArgs) -> Result<()> {
    // Tighten per-action timeouts when running as a hook handler.
    // Callers can still override via their own environment.
    if std::env::var_os("CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS").is_none() {
        // SAFETY: we are single-threaded at this point (pre-tokio spawn);
        // setting env here is safe and mirrors the dispatch env bootstrap.
        // SAFETY guarantee matches the similar pattern in wrap/env.rs.
        unsafe { std::env::set_var("CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS", "3"); }
    }
    if std::env::var_os("CLAUDINE_MESSENGER_TIMEOUT_SECONDS").is_none() {
        unsafe { std::env::set_var("CLAUDINE_MESSENGER_TIMEOUT_SECONDS", "3"); }
    }

    let raw = { /* ... existing ... */ };
    // ...
}
```

If the Rust version being used treats `set_var` as safe, drop the `unsafe` block — check against the toolchain currently in `rust-toolchain.toml`. The pattern should mirror how the existing code handles env var bootstrapping elsewhere (grep for `set_var` in `claudine/cli/src/commands/wrap/env.rs`).

- [ ] **Step 4: Run the bash-timeout test**

Run: `cargo test -p claudine-cli --test handle_commands handle_bash_action_times_out_cleanly`
Expected: PASS.

- [ ] **Step 5: Add matching messenger timeout**

In `claudine/lib/src/messaging/send.rs`, find the outbound-send functions (per-provider: Discord / Slack / Signal / WhatsApp). Each performs an `reqwest::Client` HTTP call. Wrap the send future in `tokio::time::timeout` using the same env-var pattern:

```rust
const DEFAULT_MESSENGER_TIMEOUT_SECONDS: u64 = 30;

fn send_timeout() -> std::time::Duration {
    let secs = std::env::var("CLAUDINE_MESSENGER_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MESSENGER_TIMEOUT_SECONDS);
    std::time::Duration::from_secs(secs)
}

pub async fn execute_message(
    dispatch: &Dispatch,
    receipt: &mut SendReceipt,
) -> Result<(), MessengerError> {
    match tokio::time::timeout(send_timeout(), execute_message_inner(dispatch, receipt)).await {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                provider = %dispatch.provider,
                timeout_secs = send_timeout().as_secs(),
                "messenger send timed out"
            );
            eprintln!("messenger action timed out after {}s", send_timeout().as_secs());
            Err(MessengerError::Timeout {
                timeout_secs: send_timeout().as_secs(),
                provider: dispatch.provider.to_string(),
            })
        }
    }
}
```

Extend `MessengerError` with `Timeout { timeout_secs: u64, provider: String }`; propagate through the existing `thiserror::Error` enum the same way `BashActionError::Timeout` was added.

Keep `execute_message_inner` as the renamed original body so the timeout wrapper owns the deadline.

- [ ] **Step 6: Run the full handle test module**

Run: `cargo test -p claudine-cli --test handle_commands`
Expected: all tests pass (deadline + bash timeout). The messenger timeout doesn't have its own integration test in this plan because exercising it requires a fake HTTP endpoint — that's a follow-up worth doing but not blocking.

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/actions/bash_executor.rs claudine/lib/src/messaging/send.rs \
        claudine/cli/src/commands/handle.rs claudine/cli/tests/handle_commands.rs
git commit -m "feat(claudine): per-action timeouts for bash and messenger

Bash actions gain a CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS (default 30s)
env override, tightened to 3s inside claudine handle. Messenger sends
gain CLAUDINE_MESSENGER_TIMEOUT_SECONDS (default 30s, tightened to 3s).
Both surface as new Timeout variants on their error enums so the
dispatch pipeline reports a soft-failure instead of hanging."
```

---

## Task 4: Repro fixture and regression guard

**Files:**
- Modify: `claudine/cli/tests/handle_commands.rs`

- [ ] **Step 1: Add an end-to-end regression test that replays the captured hook payload**

Use the structure of the payload Gemini fires for `AfterAgent` (turn_complete). The exact shape isn't captured in `agent-output/gemini.err` (only the timeout message is), so use a representative payload:

```rust
#[test]
fn handle_turn_complete_fast_path_completes_within_deadline() {
    let payload = serde_json::json!({
        "hook_event_name": "AfterAgent",
        "provider": "gemini",
        "session_id": "regression-turn-complete",
        "turn_number": 3,
        "elapsed_ms": 12345
    })
    .to_string();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_claudine"))
        .arg("handle")
        .arg("turn_complete")
        .arg("--provider")
        .arg("gemini")
        .env("CLAUDINE_HANDLE_DEADLINE_SECONDS", "5")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    drop(child.stdin.take());

    let start = std::time::Instant::now();
    let output = child.wait_with_output().expect("wait");
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "fast-path turn_complete should finish in <3s; took {elapsed:?}"
    );
    assert!(
        output.status.success(),
        "fast-path should exit 0, got status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
```

- [ ] **Step 2: Run it to confirm the fast path isn't regressed**

Run: `cargo test -p claudine-cli --test handle_commands handle_turn_complete_fast_path_completes_within_deadline`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/tests/handle_commands.rs
git commit -m "test(claudine): lock in handle fast-path completes under 3s

Regression guard for the turn_complete / session_end hang reported in
the 2026-04-14 capture. Runs claudine handle turn_complete --provider
gemini with a representative payload and asserts the process exits 0
in under 3s. Complements the deadline-enforcement test by covering the
normal path that was being starved by blocking actions."
```

---

## Task 5: Regression sweep

- [ ] **Step 1: Build + test both crates**

Run:
```bash
cargo test -p claudine -p claudine-cli
```
All tests must pass, including pre-existing dispatch and messenger tests.

- [ ] **Step 2: Manual verification against a real agent**

Using a dev build:

```bash
cargo build -p claudine-cli --release
export PATH="$(pwd)/target/release:$PATH"
hyperfine --runs 1 'claude -p "say hi" --output-format stream-json --dangerously-skip-permissions 2>/tmp/claude.err >/tmp/claude.out'
```

Grep the stderr for the previous hook-timeout signature:

```bash
grep -F "Hook timed out after 30000ms" /tmp/claude.err  # must return nothing
grep -F "SessionEnd hook .* failed: Hook cancelled" /tmp/claude.err  # must return nothing
```

If either signature appears, Plan 1 did not close the hang — STOP and surface to the user. Do not mark the plan complete until both are clean in a real capture.

- [ ] **Step 3: Update the captured fixtures**

Re-run the four prompts from the 2026-04-14 capture (the ones producing `claudine-output/{claude,codex,gemini,opencode}.{out,err}`). Save the new captures under a dated sub-folder so the 2026-04-14 originals remain as the "before Plan 1" baseline:

```bash
mkdir -p claudine/claudine-output/post-plan-1/
# re-run each agent prompt redirecting into post-plan-1/
```

Commit the new fixtures separately so they're identifiable:

```bash
git add claudine/claudine-output/post-plan-1/
git commit -m "chore(claudine): capture post-plan-1 baselines for plan-2 and plan-3"
```

- [ ] **Step 4: Clippy delta check**

Run:
```bash
cargo clippy -p claudine -p claudine-cli --all-targets 2>&1 | grep -cE "^error:"
```
Record the count. Compare against the pre-Plan-1 baseline (48 as of 2026-04-14). Zero net increase acceptable; any increase requires investigation before closing the plan.

---

## Out of Scope

- **Fixing the underlying blocking action (if one exists).** If the phase tracing reveals a specific action type consistently running past its deadline (e.g., every turn_complete blocking in messenger for 3+ seconds because of a slow Slack webhook), that's a separate follow-up. Plan 1 only makes the hang contained and observable — the fix for any individual slow action belongs to its own scoped change.
- **Streamlining the rest of the dispatch pipeline.** Other long-running actions (TTS, sound effects, log writes) may also benefit from deadlines but are out of scope until the phase tracing from Task 2 identifies which of them actually block in practice.
- **Claude Code's Codex tool-stdin hang** from `agent-output/codex.err`. That's a Codex-side tool router issue, not a Claudine hook issue.
- **Re-running Plan 2 / Plan 3 fixture captures.** Task 5's "post-plan-1" capture is the input corpus for Plan 2; that plan will reference it directly.
