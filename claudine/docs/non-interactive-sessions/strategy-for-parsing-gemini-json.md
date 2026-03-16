---
prompt: |-
	The Gemini CLI can output a stream of JSONL output when the `--output-format stream-json` flag is included. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.

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

last_update: 2026-03-16
---
