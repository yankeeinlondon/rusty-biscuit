# Improved Feedback: Next Spec

Actionable items with no hard external blockers. These can be started immediately.

## Ready to Implement

| Suggestion | What | Effort | Value |
| --- | --- | --- | --- |
| Auth / quota / billing badges | Add structured session badges and remediation hints for auth source, quota pressure, billing failures, and credit exhaustion when providers expose stable signals. | Medium | Gives operators faster diagnosis of why a run is failing or slowing down, especially in CI and unattended workflows. |
| More strongly typed provider protocol models | Replace more of the manual `serde_json::Value` traversal with provider-specific typed enums/structs generated or maintained from the authoritative provider contracts. | Medium | Lowers parser drift risk and makes future stream changes easier to detect and review. |

### Auth / quota / billing badges

When a wrapped session ends (or fails), Claudine should show a concise, human-readable badge like:

```
⚠ billing_error — Insufficient credits
  → Check your balance: https://console.anthropic.com/settings/billing
```

Today the infrastructure already exists in pieces:

- `ClaudeStreamParser` already detects `error_kind = "billing_error"` and extracts `error_message` (see `claudine/lib/src/stream/claude.rs:139-154`)
- `Provider::usage_dashboard_url()` already maps each provider to its billing dashboard URL (see `claudine/lib/src/events/provider.rs:514-526`)
- `BillingCapabilities` already models each provider's billing models (subscription, per-token, prepaid credits) with notes (see `claudine/lib/src/agents/model.rs:177-188`)

The gap: these pieces aren't wired together into a **structured badge** that the stderr output renderer can emit. The work is to define a `SessionBadge` type (with fields like severity, label, message, remediation URL) and produce them from the parser summary + provider capabilities. For example, when `error_kind == "billing_error"`, emit a badge with severity `error`, label "Billing", the error message, and a deep link to the right dashboard. Similar badges could cover auth failures (`"authentication failed"` already detected in `output.rs:937`) and rate-limit exhaustion (already parsed as `RateLimitInfo`).

Start with Claude and Codex (stable auth/quota signals), then extend to other providers incrementally.

### More strongly typed provider protocol models

Replace the manual `serde_json::Value` traversal patterns with proper Rust structs. Here's a concrete before/after:

**Today** (`claude.rs:139-154`, the error handler):

```rust
fn handle_error(&mut self, obj: &Value) {
    self.is_error = true;
    self.error_kind = obj
        .get("error")
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .map(String::from);
    self.error_message = obj
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(String::from);
}
```

Every field is extracted manually with `.get().and_then().and_then()` chains against `Value`. If Claude's JSON contract changes, nothing catches it until runtime.

**After** (with typed structs):

```rust
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeEvent {
    Init(ClaudeInit),
    Assistant(ClaudeAssistant),
    ContentBlockDelta(ClaudeDelta),
    Error { error: ClaudeError },
    Result(ClaudeResult),
    RateLimitEvent(ClaudeRateLimit),
    // ...
}

#[derive(Deserialize)]
struct ClaudeError {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}
```

Then the error handler becomes `let ClaudeEvent::Error { error } = serde_json::from_str(line)?;` — no manual traversal, and serde enforces the contract at deserialization time. The same pattern applies to `init`, `result`, `rate_limit_event`, etc. For Claude and Codex the authoritative JSON contracts are well-documented, so the work is mechanical: define the structs, wire them into the existing `feed_line()` method, and let the existing tests confirm behavior is preserved. Extend to Gemini/OpenCode/Goose afterward.

## Deferred On Purpose

The following ideas came up in the original draft but should stay deferred until the parser-correctness slice is complete:

- transient live progress bars
- per-provider custom checklist widgets
- aggressive session-header badge expansion
- pricing-table-driven cost estimates
- deeper OpenCode permission/question UX built only from filtered stdout
- protocol migrations and parser rewrites in the same change set

## Blocked Items

All remaining suggestions are blocked on external decisions, new architecture, or upstream changes. See [blocked.md](./blocked.md) for the full list and blocker reasons.
