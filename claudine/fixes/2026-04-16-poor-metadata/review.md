# Review — Poor Metadata in Live Semantic Rendering

**Date:** 2026-04-16
**Status:** Investigation complete
**Severity:** High (user-facing output is near-useless for Claude Code subscription users running tool-heavy tasks)

## Symptoms (from user's screenshot)

Running `claudine claude -p "Implement Phase 1 …"` produced stderr containing:

```
- Claude session ID 0142667a-bc9 · claude-opus-4-7[1m]
⚠ rate limit                         ← FALSE POSITIVE
← (tool)(successful)                 ← 12× lines like this
← (tool)(successful)
…
← (tool)(successful)

Now let me look at existing tests and fixtures …   (final stdout)
```

Three distinct problems in one session:

1. **No tool-call start lines.** Every `→ ToolName(args)` line is missing; only incoming `←` result lines appear.
2. **Tool names replaced with `(tool)`.** The humanizer fallback for "empty name" is firing on every result.
3. **`⚠ rate limit` warning renders for a subscription user** who was not actually rate-limited. The session completed all 12 tool calls successfully, so throttling did not occur.

All three regressions survive the current test suite. Each has a specific root cause and a straightforward fix.

## Affected files (quick index)

| Topic | File | Lines |
|------|------|------|
| Tool name dropped on ToolResult | `claudine/lib/src/stream/claude_semantic.rs` | 437-456, 497-505 |
| Tool call not emitted from `assistant.message.content[*]` | `claudine/lib/src/stream/claude_semantic.rs` | 165-219 |
| Assistant `ContentPart` strips tool_use fields | `claudine/lib/src/stream/protocol/claude.rs` | 104-110 |
| `content_block_delta` input_json_delta never merged into ToolCall | `claudine/lib/src/stream/claude_semantic.rs` | 221-266, 268-277 |
| Default `"rate limit"` fallback when message missing | `claudine/lib/src/stream/claude_semantic.rs` | 401-410 |
| Rate-limit suppression predicate | `claudine/cli/src/commands/wrap/live_semantic_sink.rs` | 827-838 |
| `apiKeySource` on `init` event is captured nowhere | `claudine/lib/src/stream/protocol/claude.rs` | 44-54 |
| ToolCallDisplay fallback to `(tool)` | `claudine/lib/src/stream/tool_display.rs` | 420-428, 453-459 |

## Root cause analysis

### Issue 1 — Missing tool-call *start* lines (`→`)

Real Claude Code stream output (captured in `claudine/agent-output/claude.out` and `claudine/lib/tests/fixtures/providers/claude.ndjson`) is organized around *whole* `assistant` envelopes, not content-block deltas. A turn that invokes Bash looks like:

```jsonl
{"type":"assistant","message":{"role":"assistant","content":[
  {"type":"text","text":"I'll check the existing tests."},
  {"type":"tool_use","id":"toolu_01ABC","name":"Bash","input":{"command":"ls -la"}}
]}}
{"type":"user","message":{"role":"user","content":[
  {"type":"tool_result","tool_use_id":"toolu_01ABC","content":"…","is_error":false}
]}}
```

In `claudine/lib/src/stream/claude_semantic.rs::handle_assistant_message` (lines 165-219), the parser extracts **only** `text` content parts and silently drops everything else:

```rust
for part in content {
    if part.kind.as_deref() == Some("text")
        && let Some(text) = part.text
    {
        text_parts.push_str(&text);
    }
}
```

The `ClaudeContentPart` struct in `protocol/claude.rs` (lines 104-110) only declares `kind` and `text` — there is no `id`, `name`, or `input`. Even if the handler were updated, the serde deserialization has already thrown away the tool_use fields by this point. Therefore **no `SemanticEvent::ToolCall` is ever emitted for the common `assistant` envelope path**.

`handle_content_block_start` / `handle_content_block_delta` (lines 268-277 and 221-266) handle the alternate streaming path that uses `content_block_start` + `content_block_delta` events. Real Claude Code `--print --verbose --output-format stream-json` does NOT use that path; it emits complete `assistant` messages. So the existing tool_use code is dead for wrapped runs.

**Secondary bug (also in this path):** even when `content_block_start` *does* fire, the `input_json_delta` deltas arriving afterward fall through to `ProviderExtension` (lines 249-264) and are never merged back onto the `ToolCall`. That means the tool summary would be empty anyway.

### Issue 2 — Tool name renders as `(tool)`

Both emission sites for `SemanticEvent::ToolResult` in `claude_semantic.rs` explicitly pass `name: None`:

```rust
// handle_tool_result, line 437-456
self.sink.on_semantic_event(SemanticEvent::ToolResult {
    name: None,         // <-- bug
    id: tool_id,
    status: None,
    exit_code: None,
    output,
    extra: Value::Object(extra),
});

// handle_user tool_result block, line 497-505
self.sink.on_semantic_event(SemanticEvent::ToolResult {
    name: None,         // <-- bug
    id: tool_id,
    status,
    exit_code: None,
    output,
    extra: Value::Object(extra),
});
```

Claude's `tool_result` blocks carry only `tool_use_id`, never a `name`. The ONLY way to recover the name on the result side is to look up the id in a map populated at tool_use time. **The Claude parser does not keep such a map.**

This is asymmetric with every other provider. Comparison (grep-verified in `stream/*_semantic.rs`):

| Provider | ToolResult `name:` populated? | Mechanism |
|----------|-------------------------------|-----------|
| **Claude** | **NO (always `None`)** | — |
| Codex | YES | `fields.resolved_tool_name()` |
| Gemini | YES | local `tool_name` var |
| Kimi | YES | local `tool_name` var |
| OpenCode | YES | `tool_uses: HashMap<id, (name, input)>` — resolved at result time (`opencode_semantic.rs:316-346`) |
| Qwen | YES | local `tool_name` var |

So the live sink's `ToolCallDisplay::from_result` sees an empty name and falls back to the literal string `"(tool)"` via `tool_display.rs:454-459`:

```rust
let display_name = if raw_name.is_empty() {
    "(tool)".into()
} else {
    humanize_tool_name(raw_name)
};
```

**Every single `←` line in the user's screenshot is this fallback.** It has nothing to do with the tool actually being unknown — the name is simply never wired from tool_use to tool_result in the Claude parser.

### Issue 3 — `⚠ rate limit` false positive

Two compounding problems:

**3a. Default message "rate limit" fires whenever Claude emits an advisory `rate_limit_event` with no message.**

`claude_semantic.rs:401-404`:

```rust
let message = info
    .message
    .clone()
    .unwrap_or_else(|| "rate limit".to_string());
```

And crucially, `handle_rate_limit` **always** emits `SemanticEvent::Warning` regardless of the `is_throttled` flag:

```rust
self.sink.on_semantic_event(SemanticEvent::Warning {
    message,
    extra: Value::Object(extra),
});
```

Claude Code surfaces `rate_limit_event` lines as informational milestones too (e.g. near the start of a turn, or as a usage-window reset notice). Many of these carry `is_throttled: false` and either no message or a neutral phrasing. Treating them all as warnings is wrong.

This is confirmed by a real captured event in `~/.claudine/logs/2026-04-16.jsonl`:

```json
{
  "semantic_event": {
    "type": "warning",
    "message": "rate limit",
    "extra": { "line_num": 8, "provider": "claude", "raw_kind": "rate_limit_event" }
  },
  "notification_type": "warning",
  "error": "rate limit"
}
```

The `is_throttled` field is not present — meaning the stream payload lacked it or set it to null/false. Either way, the Warning fired and then rendered.

**3b. The suppression predicate relies on the wrong signal.**

`cli/src/commands/wrap/live_semantic_sink.rs:827-838`:

```rust
fn is_suppressed_claude_rate_limit(provider: Provider, extra: &Value) -> bool {
    if provider != Provider::Claude { return false; }
    let raw_kind = extra.get("raw_kind").and_then(Value::as_str).unwrap_or("");
    if raw_kind != "rate_limit_event" { return false; }
    std::env::var("ANTHROPIC_API_KEY")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
}
```

The presence of `ANTHROPIC_API_KEY` **in claudine's own environment** is a noisy proxy for "user is on API-key auth." The user's shell may have the variable set for unrelated tooling; the wrapped `claude` process may still end up using subscription auth (Claude Code's own resolution order). The authoritative signal is Claude Code's own `init` event, which carries `apiKeySource`. In the captured fixture (line 5):

```
"apiKeySource":"ANTHROPIC_API_KEY"
```

Values observed in the wild:
- `"ANTHROPIC_API_KEY"` — env-var auth (warnings should render)
- `"/login managed key"` — `claude login` subscription auth (warnings should be suppressed)
- `"none"` — no auth

**Fix:** capture `apiKeySource` on init and use it as the first-class signal, falling back to the env var only when `apiKeySource` is absent.

### Issue 4 — Why the existing tests did not catch any of this

Grep-audited:

1. **`tool_use_and_result_emit_typed_events`** (`claude_semantic.rs:892-920`) — feeds only top-level `{"type":"tool_use", …}` and `{"type":"tool_result", …}` lines. Real Claude Code **never** emits those at the top level; they always arrive nested inside `assistant` / `user` envelopes. The test exercises a code path that the wrapped binary never takes.

2. **`user_event_routes_tool_result_to_semantic_tool_result`** (`claude_semantic.rs:1166-1181`) — asserts the event *kind* is `tool_result`. It does NOT assert the emitted `name` field is populated, so the `name: None` bug passes.

3. **No test feeds an `assistant` envelope whose `content` array contains a `tool_use` block.** The closest is `claude_assistant_deserializes_flat_content` in `protocol/claude.rs`, which uses a text-only content array. The bug is invisible to every assistant-message test.

4. **`claude_fixture_full_replay_produces_no_provider_extensions`** (`claude_semantic.rs:1303-1335`) — the sole "realistic" replay test uses `claude.ndjson`, which is a 7-line billing_error fixture with no tool usage at all.

5. **`claude_rate_limit_warning_suppressed_when_anthropic_api_key_unset`** (`live_semantic_sink.rs:1883-1902`) — proves the predicate works when the env var is *unset*. It does not probe the far more common regression: env var set for unrelated reasons, or `apiKeySource` not `ANTHROPIC_API_KEY`.

The test plan in `features/2026-04-16-leveraging-logs/` addresses log reporting; it did not regress-check tool-call rendering for Claude's assistant-envelope path.

## Other providers — same class of bugs?

- **Codex / Gemini / Kimi / Qwen / OpenCode:** all populate `name` on ToolResult events. None show `(tool)` in live output.
- **Codex and Gemini also emit a full tool event pair** (start + end) because their wire formats have distinct `function_call` / `tool_use` + `function_call_output` / `tool_result` events.
- **OpenCode is the reference design for this class of problem:** it maintains `tool_uses: HashMap<id, (name, input)>` (`opencode_semantic.rs:316-346`) so it can recover the name at result time even when the `tool_result` payload omits it. Claude should copy this pattern.
- **No other provider synthesizes a generic "rate limit" Warning with the same false-positive profile.** Codex/OpenCode rate-limit handling is driven by explicit `usage_limit_reached` / `zai-coding-plan` error payloads that carry real messages (see `features/2026-04-16-leveraging-logs/example-of-usage-limit.txt` for an OpenCode usage-limit example).

Net: this review is specifically a Claude-parser and Claude-suppression issue. The other providers are OK in this narrow area.

## Recommended fixes (minimum viable)

### Fix A — Emit `ToolCall` from `assistant.message.content[*]`

1. In `protocol/claude.rs`, widen `ClaudeContentPart` to include `id`, `name`, `input`, and `tool_use_id` with `#[serde(default)]`. A single struct can represent text, tool_use, and tool_result parts; pick the active shape by `kind`.
2. In `claude_semantic.rs::handle_assistant_message`, when iterating `content`, dispatch each part by `kind`:
   - `text` → append to `text_parts` (existing behavior)
   - `tool_use` → call `handle_tool_use(part.into_tool_use(), raw_kind)`
   - anything else → `ProviderExtension` with kind `assistant.content.<other>`
3. In `handle_user`, do the same by part kind — `tool_result` continues to flow through the existing tool_result path; `tool_use` (rare but technically legal in some playback formats) should also dispatch to `handle_tool_use`.

This restores `→ Bash(bash ls -la)` lines for every tool call.

### Fix B — Resolve ToolResult `name` via id-to-name cache

Mirror the OpenCode pattern (`opencode_semantic.rs:316-346`):

1. Add a field to `ClaudeSemanticStreamParser`:
   ```rust
   tool_uses: HashMap<String, (Option<String>, Option<Value>)>, // id -> (name, input)
   ```
2. Populate in `handle_tool_use` (after the existing emission):
   ```rust
   if let Some(id) = &tool_id {
       self.tool_uses.insert(id.clone(), (tool_name.clone(), tool_input.clone()));
   }
   ```
   (Ensure `tool_input` clone happens BEFORE `take_input()` so the cached input is not empty — or clone before the `take`.)
3. In `handle_tool_result` and the `handle_user::tool_result` branch, look up the cached name by id:
   ```rust
   let cached_name = tool_id.as_ref()
       .and_then(|id| self.tool_uses.remove(id))
       .and_then(|(name, _)| name);
   ```
4. Pass `name: cached_name` into both `SemanticEvent::ToolResult` emissions.

### Fix C — Merge `input_json_delta` into the in-flight ToolCall

Low priority for the immediate fix (Fix A alone restores tool call visibility), but still worth closing:

1. Track `in_flight_tool_call: Option<ClaudeToolUse>` keyed by `content_block_start`'s index/id.
2. On `content_block_delta` with `kind == "input_json_delta"`, accumulate `partial_json` fragments.
3. On `content_block_stop` (or the matching `tool_result`), parse the accumulated JSON and patch it onto the cached tool_use before emission.

Alternately — and simpler — **only emit `ToolCall` from `assistant.message.content[*]`** (Fix A) and treat `content_block_start` as an informational signal that a tool call is beginning, without emitting a full `ToolCall` until the parent `assistant` envelope arrives. The `assistant` envelope always arrives with complete `input`; the streaming start/delta is just a live-progress artifact.

### Fix D — Classify `rate_limit_event` correctly

1. In `ClaudeRateLimit`, the `is_throttled` field is already modeled. Gate the Warning on it:
   ```rust
   let is_throttled = info.is_throttled.unwrap_or(false);
   if is_throttled {
       self.sink.on_semantic_event(SemanticEvent::Warning { … });
   } else {
       // Informational — route to ProviderExtension with kind "rate_limit_event"
       // so dispatch and JSONL logging still see it, but nothing renders.
       self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
           provider: Provider::Claude,
           kind: "rate_limit_event".into(),
           payload: raw,
       });
   }
   ```
2. Never synthesize `"rate limit"` as the message. If `info.message.is_none()`, emit a more descriptive default constructed from `retry_after_ms` / `is_throttled`, or better, omit the Warning entirely (see above).

### Fix E — Use `apiKeySource` as the auth-mode signal

1. Add `api_key_source: Option<String>` to `ClaudeInit` with `#[serde(default, alias = "apiKeySource")]`.
2. Cache it on the parser: `api_key_source: Option<String>` initialized to `None`, updated in `handle_init`.
3. Propagate it into the `SessionStart` event's `extra` so downstream consumers (including the live sink) can read it.
4. Update `is_suppressed_claude_rate_limit` to prefer the init signal:
   ```rust
   // Primary signal: Claude Code's own apiKeySource from init.
   match cached_api_key_source.as_deref() {
       Some("/login managed key") | Some("none") => return true, // subscription → suppress
       Some("ANTHROPIC_API_KEY")                  => return false, // real key → show
       _ => { /* fall through to env-var heuristic */ }
   }
   // Legacy fallback: env var in this process.
   std::env::var("ANTHROPIC_API_KEY").map(|v| v.trim().is_empty()).unwrap_or(true)
   ```
5. Caching the value on the sink requires a new `SemanticEvent::SessionStart` read-path: either extend the sink's session-state tracking to also record the auth source, or route it via `extra`. The first is cleaner.

## Test additions (regression-proof)

All four of the following tests would have caught the visible regressions. They should be added under `claudine/lib/src/stream/claude_semantic.rs::tests`:

```rust
#[test]
fn assistant_envelope_with_tool_use_block_emits_tool_call() {
    let (sink, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"init","session_id":"s","model":"m"}"#).unwrap();
    parser.feed_line(
        r#"{"type":"assistant","message":{"role":"assistant","content":[
            {"type":"text","text":"let me check"},
            {"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls"}}
        ]}}"#
    ).unwrap();
    let events = sink.snapshot();
    let has_tool_call = events.iter().any(|e| matches!(e,
        SemanticEvent::ToolCall { name: Some(n), .. } if n == "Bash"));
    assert!(has_tool_call, "assistant.content[tool_use] must emit ToolCall; got {:?}", sink.kinds());
}

#[test]
fn tool_result_resolves_name_from_tool_use_id_cache() {
    let (sink, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"init","session_id":"s","model":"m"}"#).unwrap();
    parser.feed_line(
        r#"{"type":"assistant","message":{"content":[
            {"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"/etc/hosts"}}
        ]}}"#
    ).unwrap();
    parser.feed_line(
        r#"{"type":"user","message":{"content":[
            {"type":"tool_result","tool_use_id":"toolu_1","content":"# hosts","is_error":false}
        ]}}"#
    ).unwrap();
    let events = sink.snapshot();
    let result = events.iter().find_map(|e| match e {
        SemanticEvent::ToolResult { name, .. } => Some(name.clone()),
        _ => None,
    }).expect("expected a ToolResult event");
    assert_eq!(result.as_deref(), Some("Read"),
        "ToolResult must carry the name resolved from the tool_use_id cache");
}

#[test]
fn rate_limit_event_without_throttle_flag_does_not_warn() {
    let (sink, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"init","session_id":"s","model":"m"}"#).unwrap();
    parser.feed_line(r#"{"type":"rate_limit_event","is_throttled":false}"#).unwrap();
    let has_warning = sink.snapshot().iter().any(|e| matches!(e, SemanticEvent::Warning { .. }));
    assert!(!has_warning, "advisory rate_limit_event must not surface as a Warning");
}

#[test]
fn init_captures_api_key_source_and_routes_through_session_start() {
    let (sink, mut parser) = new_parser();
    parser.feed_line(
        r#"{"type":"system","subtype":"init","session_id":"s","model":"m","apiKeySource":"/login managed key"}"#
    ).unwrap();
    let extra = match &sink.snapshot()[0] {
        SemanticEvent::SessionStart { extra, .. } => extra.clone(),
        other => panic!("expected SessionStart; got {other:?}"),
    };
    assert_eq!(
        extra.get("api_key_source").and_then(|v| v.as_str()),
        Some("/login managed key"),
        "SessionStart extra must expose apiKeySource so downstream suppression can use it"
    );
}
```

Plus at the sink level:

```rust
#[test]
#[serial_test::serial]
fn claude_rate_limit_suppressed_when_api_key_source_is_login_managed() {
    // Env var set for unrelated reasons — must not defeat subscription-side suppression.
    let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-some-unrelated-key");
    let mut sink = /* … */;
    // Prime the sink with a SessionStart whose extra says apiKeySource = "/login managed key"
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s".into()),
        model: Some("m".into()),
        extra: json!({"api_key_source": "/login managed key"}),
    });
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limit".into(),
        extra: json!({"raw_kind": "rate_limit_event"}),
    });
    assert!(captured_lines.lock().unwrap().iter().all(|l| !l.contains("rate limit")),
        "subscription users must not see rate-limit warnings even when ANTHROPIC_API_KEY is set in their shell");
}
```

## Prioritization

| Priority | Fix | Rationale |
|----------|-----|-----------|
| P0 | Fix A (emit ToolCall from `assistant.content`) | Tool call start lines are the single highest-value stderr signal; users can't supervise an agent without them. |
| P0 | Fix B (resolve name via id cache) | Eliminates every `(tool)` in output. Small diff, mirrors OpenCode. |
| P1 | Fix D (gate rate-limit Warning on `is_throttled`) | Removes the loudest false positive. Low risk. |
| P1 | Fix E (`apiKeySource` as primary suppression signal) | Fixes the subscription-user edge case where env var is set for unrelated tools. |
| P2 | Fix C (merge `input_json_delta` into in-flight ToolCall) | Only matters if real-time streaming is re-enabled; currently dead code path. |

## Appendix — references

- Real Claude Code stream format (captured): `claudine/agent-output/claude.out`
- Research docs on stream shapes: `claudine/docs/research/non-interactive-sessions/claude.md` (lines 347-395)
- OpenCode's id-to-name cache (reference implementation): `claudine/lib/src/stream/opencode_semantic.rs:316-346`
- Captured false-positive warning from today's log: `~/.claudine/logs/2026-04-16.jsonl` (grep for `"semantic_kind":"warning"`)
- Current suppression predicate: `claudine/cli/src/commands/wrap/live_semantic_sink.rs:827-838`
- Existing coverage gap: `claudine/lib/tests/fixtures/providers/claude.ndjson` contains only a billing_error scenario; no tool-use scenarios exist in any claude fixture.
