---
prompt: |-
	The Qwen CLI can output a stream of JSONL output when the `--output-format stream-json` flag is included. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.

    Here's an example of the JSONL data you might get in a request:

    ```json
    {
        "type": "init",
        "timestamp": "2026-03-16T06:30:32.748Z",
        "session_id": "b5b53246-4d23-42e5-9adb-71ccaefd09ba",
        "model": "auto-gemini-3"
    }
    {
        "type": "message",
        "timestamp": "2026-03-16T06:30:32.748Z",
        "role": "user",
        "content": "Hi. My name is Ken"
    }
    {
        "type": "message",
        "timestamp": "2026-03-16T06:30:41.946Z",
        "role": "assistant",
        "content": "Hello Ken. I am Gemini CLI, a senior software engineer assistant. How can I help you with the `rusty-biscuit",
        "delta": true
    }
    {
        "type": "message",
        "timestamp": "2026-03-16T06:30:44.060Z",
        "role": "assistant",
        "content": "` monorepo or the `claudine` package today?",
        "delta": true
    }
    {
        "type": "result",
        "timestamp": "2026-03-16T06:30:48.277Z",
        "status": "success",
        "stats": {
            "total_tokens": 33128,
            "input_tokens": 32983,
            "output_tokens": 87,
            "cached": 0,
            "input": 32983,
            "duration_ms": 15530,
            "tool_calls": 0
        }
    }
    ```

    - This metadata can be used to present metadata to the user on STDERR when they are executing a non-interactive command.
    - This metadata can be used to enhance the data we're providing to our logging platform

    Your task is to:
    
    - research other examples online and fill in any other missing details not self-evident from the example data
    - determine how best to feed the metadata to logging and non-interactive sessions.

last_updated: 2026-03-16
---
# Qwen CLI's JSON Output

## Key Findings

**Qwen CLI is a fork of Gemini CLI** — the stream-json format is structurally identical. Same 6 event types, same field names, same stats shape.

### Event types beyond the example you provided

| Type | Not in your example | Purpose |
|------|-------------------|---------|
| `tool_use` | Yes | Tool invocation with `tool_name`, `tool_id`, `parameters` |
| `tool_result` | Yes | Tool completion with `tool_id`, `status`, optional `output`/`error` |
| `error` | Yes | Non-fatal warnings (e.g. loop detection) with `severity`, `message` |

### Stats nuance
`input_tokens` (32,983) and `input` (32,983) are **not duplicates** — `input_tokens` is the full prompt token count, `input` is the non-cached portion. In your example they're equal because `cached` is 0.

## Strategy for Integration

The document at `claudine/docs/non-interactive-sessions/qwen-json.md` covers:

1. **STDERR metadata** — compact session info on `init`, warnings on `error`, token/duration summary on `result`
2. **Logging** — synthesize one Claudine JSONL event at session end with normalized `token_usage`, `model`, `session_id`, and tool timeline
3. **Shared parser** — since the format is identical to Gemini's, extract a shared `StreamJsonParser` for both providers
4. **Missing implementation** — `QwenWrapper` needs `apply_output_format` (identical to `GeminiWrapper`'s)
5. **Output-mode rules** — force `stream-json` internally by default, respect explicit `--output text/json/stream`
