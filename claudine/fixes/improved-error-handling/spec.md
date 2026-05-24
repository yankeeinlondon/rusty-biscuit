---
created: 2026-05-12
provider: all
severity: bug
related_research:
  - claudine/docs/research/agent-cli/opencode.md
related_memory:
  - feedback_claudine_opencode_false_hang
  - feedback_claudine_timeout_nomenclature
related_fixes:
  - claudine/fixes/2026-05-12-opencode-stderr-returns
---

# Compose: Honest Error Classification and Rate-Limit-Aware Loop Iteration

## The Problem We Are Solving

A real, observed non-interactive `compose` run against OpenCode + kimi-k2.6:

1. Phase 1 of a 6-phase plan completes successfully. The provider attaches a trailing `⚠ Rate Limit — Usage limit reached for k2p6` notice — i.e. `summary.rate_limit.is_throttled = true` with a future `reset_at`.
2. The loop iterates to Phase 2 without inspecting the rate-limit signal. Phase 2 spawns the provider immediately.
3. Phase 2 burns through whatever residual quota the sliding window had, runs a handful of tool calls, then the LLM provider silently stalls on a 429 retry storm. No bytes flow.
4. The watchdog correctly fires `step_timeout` after 30 minutes of silence and kills the child with exit code 1.
5. The top-level error reported to the user is:

   ```
   CompositionError: composition failed
   ┃ invalid loop definition: provider exited with code 1
   ```

   This is a **wrong error**. The user's `loop:` frontmatter is fine. The real cause is `step_timeout` after a hung LLM retry — itself a downstream effect of the rate-limit signal the loop ignored one phase earlier.

Two distinct defects converge to produce this experience:

- **(A) Classification bug.** `CompositionError::LoopInvalid` is overloaded. Its `#[error("invalid loop definition: {0}")]` `Display` impl is only correct for malformed `loop:` frontmatter, but it is also used as a catch-all wrapper whenever a loop iteration's child process exits non-zero — even when the wrapper layer has already written a precise `exit_reason` (e.g. `step_timeout`) to the iteration's JSONL `session_end` row. The accurate cause is recorded once at the wrap layer and then immediately discarded by the loop layer.
- **(B) Rate-limit-blind iteration.** The loop's success/failure decision reads `outcome.exit_code` and nothing else. Even when the iteration's `StreamExecutionSummary` reports `rate_limit.is_throttled = true` with a `reset_at`, the next iteration launches immediately. The very signal we already parse and carry is then dropped on the floor at the loop boundary.

Either defect on its own produces user-hostile failure modes. Together they produce a doubly-misleading outcome: the user is told a non-existent fault (loop definition is invalid) caused by a state that was preventable (don't launch the next iteration while the provider is rate-limited).

## What We Are Building

### Goal

Make `compose --loop` error reporting **honest** and **rate-limit-aware**, so that:

- A non-zero child exit is reported as what it actually is — `step_timeout`, watchdog wall-clock breach, signal, or provider exit — never as "invalid loop definition".
- Rate-limit signals on iteration N feed the iteration N+1 launch decision. The default is to pause until `reset_at` (or abort cleanly with a structured error); behavior is overridable from the loop frontmatter.
- The two-rule watchdog architecture and existing precedence (CLI > frontmatter > env > default) for `timeout` / `step_timeout` is **unchanged**.
- Existing semantics for malformed `loop:` frontmatter (`LoopInvalid`'s legitimate use) are preserved.

### Non-Goals

- **Do not change watchdog timeouts.** A long Rust test legitimately takes 10+ minutes. We are reclassifying outcomes and gating launches, not making the kill decision more aggressive. (See [`feedback_claudine_opencode_false_hang`](../../../).)
- **Do not infer rate limits we did not parse.** The existing stdout/stderr parsers already emit `RateLimitInfo`; this work consumes that signal, it does not invent new detection.
- **Do not block on the deeper OpenCode 429 retry-storm heartbeat problem.** That belongs to its own ticket (and to the [stderr-as-first-class-source spec](../../2026-05-12-opencode-stderr-returns/spec.md)). The rate-limit-aware loop is sufficient to avoid the failure mode described above without solving the upstream retry-storm visibility gap.

### Required Behavior Changes

#### 1. Stop overloading `LoopInvalid`

In `claudine/cli/src/commands/compose.rs:469-476` and `:779-786`, today's code is:

```rust
Ok(claudine::composition::LoopIterationOutput::failure(
    "",
    outcome.exit_code,
    claudine::composition::CompositionError::LoopInvalid(format!(
        "provider exited with code {}",
        outcome.exit_code
    )),
))
```

Replace the `LoopInvalid` wrap with a new, runtime-fault variant on `CompositionError`:

```rust
/// A loop iteration's provider exited non-zero. This is a runtime failure
/// distinct from `LoopInvalid` (frontmatter problem) and from
/// `LoopInterrupted` (Ctrl+C).
#[error("loop iteration {iteration} of {prompt_path}: {reason} (exit code {exit_code})",
        prompt_path = prompt_path.display())]
LoopIterationFailed {
    iteration: usize,
    prompt_path: PathBuf,
    exit_code: i32,
    /// Human-readable cause derived from the iteration's session_end row
    /// (e.g. "step_timeout", "wall-clock timeout", "signal SIGTERM",
    /// "provider exited non-zero"). Never empty.
    reason: String,
    /// The structured exit_reason from the iteration's JSONL session_end
    /// row when one is present (e.g. `step_timeout`, `wall_clock_timeout`,
    /// `signal`, `provider_exit`). `None` when no row was written.
    exit_reason: Option<String>,
},
```

The compose loop callback must populate `reason` and `exit_reason` from the iteration's `session_end` row when available. The wrap layer already writes this row with `extra.exit_reason` (covered by `wrap_commands.rs:5016-5017`); the loop layer must read it. When no row is present, fall back to a generic `"provider exited non-zero"` with `exit_reason: None`. The `iteration` value must reflect the actual phase / iteration number that failed, so the user can correlate the error with the JSONL trail.

`LoopInvalid` reverts to its documented meaning: malformed `loop:` frontmatter caught at parse/validate time. No code path outside the loop parser/validator should construct it.

`LoopInterrupted` is unchanged — Ctrl+C still routes through that variant with the conventional 130 exit code.

#### 2. Read the iteration's rate-limit signal

After each iteration, the loop callback already has the `outcome.summary` in scope. Inspect `summary.rate_limit`:

| Condition | Action |
|---|---|
| `is_throttled != Some(true)` | No rate-limit gate; proceed to next iteration as today |
| `is_throttled == Some(true)` with `reset_at` in the future | Apply the configured `on_rate_limit` action (default: `pause`) |
| `is_throttled == Some(true)` without `reset_at` (or `reset_at` already past) | Apply the configured `on_rate_limit` action (default: `abort`); without a reset clock, pausing is unbounded |

`reset_at` arithmetic uses `Utc::now()` at the moment the loop callback completes the failed/throttled iteration. A small safety margin (e.g. +5s) should be added when pausing, because providers often return `429` for a moment after the nominal reset.

#### 3. New loop frontmatter knob: `on_rate_limit`

Add an optional field on the loop definition (parsed by `loop_config.rs`):

```yaml
loop:
    until: "phase > total_phases"
    action: increment(phase)
    on_rate_limit: pause   # default; values: pause | abort | continue
```

Semantics:

- `pause` (default) — sleep until `reset_at + 5s`, then proceed to the next iteration. If `reset_at` is missing or already past, behave as `abort` (no infinite sleep). While paused, emit a styled INFO line: `⏸  Rate limit hit on iteration N; resuming at <local time> (<duration>)`. Pause is interruptible by SIGINT, which routes through `LoopInterrupted` exactly like today.
- `abort` — fail fast. Emit `CompositionError::LoopRateLimited { iteration, reset_at, provider_id, model_id, message }` (new variant). This stops the loop with a structured error that includes the reset time, the offending provider/model identity (when known), and the rate-limit message that came from the provider — none of which `LoopInvalid` could carry.
- `continue` — proceed to the next iteration without pausing. Reserved for cases where the user knows the cap is a soft, per-request signal that won't recur. **Not recommended; documented but not the default.**

The CLI must accept `--on-rate-limit <pause|abort|continue>` as an override on `compose`, with the usual precedence (CLI > frontmatter > default).

#### 4. New `CompositionError` variant for abort path

```rust
/// A loop iteration completed but reported a provider rate limit, and
/// `on_rate_limit: abort` was selected (or no `reset_at` was available
/// to safely pause).
#[error(
    "loop halted at iteration {iteration} of {prompt_path}: provider rate limited{provider}{reset}{detail}",
    prompt_path = prompt_path.display(),
    provider = .provider.as_ref().map(|p| format!(" ({p})")).unwrap_or_default(),
    reset = .reset_at.as_ref().map(|r| format!("; resets at {}", r.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S"))).unwrap_or_default(),
    detail = .message.as_ref().map(|m| format!("\n  ↳ {m}")).unwrap_or_default()
)]
LoopRateLimited {
    iteration: usize,
    prompt_path: PathBuf,
    provider: Option<String>,
    model: Option<String>,
    reset_at: Option<DateTime<Utc>>,
    message: Option<String>,
},
```

Exit code for this variant: distinguish from `LoopIterationFailed`. Suggest `75` (`EX_TEMPFAIL` from `sysexits.h`) so shell wrappers can recognize and retry-after-cool-off, separately from a generic non-zero exit. The exact value should be added to the constant table in `claudine/cli/src/output/exit_codes.rs` (or wherever exit-code mappings live) and tested.

#### 5. Show watchdog cause at the top, not the bottom

When `LoopIterationFailed { exit_reason: Some("step_timeout"), .. }` is rendered through the existing `error_walker.rs` chain, the first user-visible line should be the actionable cause, not the generic wrapper. Example target output for the recorded incident:

```
Error: loop iteration 2 of fixes/.../plan.md: step_timeout after 30m of stream silence (exit code 1)
       ↳ A step boundary was still open and the child produced no bytes for 30m 0s.
         The parent agent may have been waiting on a parallel subagent that has not yet returned.
       ↳ See: <jsonl path> session_end row for full detail.
```

The walker should pull the watchdog detail message from the iteration's `session_end` row's `extra.exit_reason_detail` (or equivalent) when present, and fall back to `reason` only.

### What Must Not Change

- The two-rule watchdog (`timeout` wall-clock + `step_timeout` silence) and the precedence between CLI / frontmatter / env / default.
- Existing precedence and naming around timeouts — keep `step_timeout` everywhere it appears today (per [`feedback_claudine_timeout_nomenclature`](../../../)). Internal Rust symbols may stay "watchdog".
- `LoopInterrupted` (Ctrl+C) routing and the conventional `130` exit code.
- The wrap layer's `session_end` JSONL contract (`extra.exit_reason=step_timeout` etc.). This spec consumes those rows; it does not change them.
- The byte-heartbeat refresh on stderr lines and any provider-specific bridge logic (e.g. OpenCode's bus filter from [the 2026-05-12 fix](../../2026-05-12-opencode-stderr-returns/spec.md)).
- Existing rate-limit detection in parsers — the `RateLimitInfo` we read here is already populated by stdout NDJSON parsers and the OpenCode stderr bridge today.

## Gotchas Worth Repeating Up Front

- **Exit code 0 with a rate-limit trailer is a real scenario.** The recorded incident's Phase 1 completed with `exit_code == 0` and `rate_limit.is_throttled == true`. The new check must run for every iteration regardless of exit code, not only on failures.
- **`reset_at` may be in the past by the time we read it.** Always compare to `Utc::now()` after reading. A past reset means "we don't have a usable wait window", which is treated as `abort` even under `on_rate_limit: pause`.
- **Pausing must be SIGINT-interruptible.** Use `tokio::time::sleep` or `std::thread::sleep` with the same interrupt plumbing the rest of the loop uses; do not block on a raw `std::thread::sleep` that swallows Ctrl+C.
- **`LoopInvalid` is still legitimate.** Frontmatter parse errors must continue to produce `LoopInvalid` from the validator — only the runtime-fault wrappers in `compose.rs` move off of it.
- **Iteration numbering.** The `iteration` field in `LoopIterationFailed` / `LoopRateLimited` must match the user-facing phase counter (1-based), not an internal zero-based index. Confirm against the loop engine's existing counter.
- **Multi-provider sessions.** A rate-limit signal carries `provider_id` and `model_id` when known. Carry both into the structured error and the abort message; failing to attribute the cap to a specific model is the most common user complaint about today's rate-limit surfacing.
- **Existing rate-limit telemetry must keep flowing.** The `StderrDiagnostics::rate_limit_events` counter, the `SessionBadge` of category `RateLimit`, and the `provider_summary.rate_limit` fields in the JSONL row are all preserved. This spec adds a loop-level decision; it does not replace the per-iteration trailers.
- **The user has observed this exact failure with OpenCode/kimi.** The acceptance criteria below include reproducing that incident with a synthetic stderr fixture; do not skip it.

## Files Most Likely to Change

- `claudine/lib/src/composition/error.rs` — add `LoopIterationFailed` and `LoopRateLimited` variants; update `BlockError` impl for both; update `Display`/`#[error]` strings; update `ErrorHeader` paths in the block renderer
- `claudine/lib/src/composition/loop_config.rs` — parse `on_rate_limit` from the loop frontmatter; default to `pause`; validate against `pause|abort|continue`
- `claudine/lib/src/composition/loop_engine.rs` — between iterations, consult `outcome.summary.rate_limit` and apply the configured action; emit the pause INFO line; honor SIGINT during pause
- `claudine/cli/src/commands/compose.rs` — at lines 469-476 and 779-786, replace `CompositionError::LoopInvalid(format!("provider exited..."))` with `LoopIterationFailed { iteration, prompt_path, exit_code, reason, exit_reason }`; populate `reason`/`exit_reason` from the iteration's `session_end` row
- `claudine/cli/src/commands/compose.rs` (CLI flags) — add `--on-rate-limit <pause|abort|continue>` with the standard precedence
- `claudine/cli/src/output/error_walker.rs` — render `LoopIterationFailed` and `LoopRateLimited` cleanly with watchdog detail surfaced at the top
- `claudine/cli/src/output/exit_codes.rs` (or wherever exit-code constants live) — add the `LoopRateLimited` → `75` mapping
- `claudine/lib/src/stream/exec_summary.rs` (or the file that owns `StreamExecutionSummary` / `RateLimitInfo`) — expose a small helper (e.g. `RateLimitInfo::active(now) -> RateLimitState { Inactive | ActiveUntil(reset) | ActiveNoReset }`) so the loop engine doesn't reimplement reset-time arithmetic
- `claudine/cli/tests/wrap_commands.rs` — extend the existing rate-limit fixtures to drive a multi-iteration `compose --loop` and assert the new behavior

## Required Documentation Updates (Do Not Skip)

- `claudine/docs/topics/compose-loops.md` (or the closest existing equivalent) — document the `on_rate_limit` knob, the default, and the three semantics. Include a worked example of the pause path.
- `claudine/docs/topics/timeouts.md` — add a callout that runtime iteration failures are no longer labeled "invalid loop definition"; cross-link to the new variant.
- `.claude/skills/claudine/SKILL.md` (or a sibling page) — add a short paragraph: "When the loop iteration child exits non-zero, the cause comes from `session_end.extra.exit_reason`, not from `LoopInvalid`. `LoopInvalid` is reserved for frontmatter validation failures only."
- Update the README or examples that show `compose --loop` to include the new flag.

## Acceptance Criteria

A. **Honest classification.**
- A composition whose iteration is killed by `step_timeout` produces an error whose first line names `step_timeout` (or `wall-clock timeout` for the other watchdog rule), includes the iteration number, and includes the watchdog's detail message. The phrase "invalid loop definition" must not appear.
- A composition with malformed `loop:` frontmatter still produces `LoopInvalid` (no regression on this path).
- A regression test in `wrap_commands.rs` extends the existing `step_timeout` fixture to run under `compose --loop` and asserts the surface error is `LoopIterationFailed` with `exit_reason == Some("step_timeout")`.

B. **Rate-limit-aware iteration.**
- With default `on_rate_limit: pause`, a synthetic OpenCode stderr fixture that emits a `usage_limit_reached` trailer with `reset_at = now + 3s` on iteration 1 causes the loop to pause ~3s and then run iteration 2, with a single INFO line announcing the pause.
- With `on_rate_limit: abort`, the same fixture causes the loop to halt with `LoopRateLimited`, exit code `75`, and an error message that includes the provider id, model id, and `reset_at` rendered in local time.
- A fixture without `reset_at` and `on_rate_limit: pause` still aborts cleanly (no infinite sleep); the abort message explains why.
- SIGINT during a pause produces `LoopInterrupted` with exit code 130 (matching current Ctrl+C behavior).

C. **Backwards compatibility.**
- All existing `wrap_commands.rs` / `sequence_cli.rs` tests pass without modification.
- Existing JSONL session_end schema is unchanged. Existing `SessionBadge`s and `provider_summary.rate_limit` blobs continue to flow.

## References

- The recorded incident: a non-interactive `compose` against OpenCode + kimi-k2.6 on 2026-05-12 that produced `invalid loop definition: provider exited with code 1` after a 30-minute silent stall in iteration 2. The diagnosis traced the labeling to `compose.rs:472-475` and `:782-785`, and the underlying cause to a rate-limit trailer dropped at the iteration boundary.
- Related fix (do not break): [`claudine/fixes/2026-05-12-opencode-stderr-returns`](../2026-05-12-opencode-stderr-returns/spec.md) — adds activity heartbeats from stderr; complementary, not a substitute. That work makes the watchdog smarter about *what counts as activity*; this work makes the loop smarter about *what to do when the iteration completes (or fails)*.
- Memory: `feedback_claudine_opencode_false_hang` — silence ≠ hang for OpenCode subagent work. Relevant because pausing on rate-limit must not increase the chance of an aggressive misclassification elsewhere.
- Memory: `feedback_claudine_timeout_nomenclature` — keep "timeout" wording on user-facing surfaces; internal symbols may stay "watchdog".
- Source: `claudine/cli/src/commands/compose.rs:469-476`, `:779-786` — the two call sites where `LoopInvalid` is misused today.
- Source: `claudine/lib/src/composition/error.rs:226-243` — current `LoopInvalid` / `LoopLimitExceeded` / `InvalidAction` shapes; the new variants live alongside.
- Source: `claudine/lib/src/stream/logs/opencode/errors.rs:548-618` — `RateLimitInfo`, `render_rate_limit_message`, and `merge_rate_limit`; consumed by this work, unchanged.
- Source: `claudine/cli/tests/wrap_commands.rs:4372-4438` — existing rate-limit fixture pattern to extend for the new loop tests.
