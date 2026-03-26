---
prompt: "The Opencode CLI can output a stream of JSONL output when the `--output-format stream-json` flag is included. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.\n\n  Here's an example of the JSONL data you might get in a request:\n\n  ```json\n  {\n      \"type\": \"init\",\n      \"timestamp\": \"2026-03-16T06:30:32.748Z\",\n      \"session_id\": \"b5b53246-4d23-42e5-9adb-71ccaefd09ba\",\n      \"model\": \"auto-gemini-3\"\n  }\n  {\n      \"type\": \"message\",\n      \"timestamp\": \"2026-03-16T06:30:32.748Z\",\n      \"role\": \"user\",\n      \"content\": \"Hi. My name is Ken\"\n  }\n  {\n      \"type\": \"message\",\n      \"timestamp\": \"2026-03-16T06:30:41.946Z\",\n      \"role\": \"assistant\",\n      \"content\": \"Hello Ken. I am Gemini CLI, a senior software engineer assistant. How can I help you with the `rusty-biscuit\",\n      \"delta\": true\n  }\n  {\n      \"type\": \"message\",\n      \"timestamp\": \"2026-03-16T06:30:44.060Z\",\n      \"role\": \"assistant\",\n      \"content\": \"` monorepo or the `claudine` package today?\",\n      \"delta\": true\n  }\n  {\n      \"type\": \"result\",\n      \"timestamp\": \"2026-03-16T06:30:48.277Z\",\n      \"status\": \"success\",\n      \"stats\": {\n          \"total_tokens\": 33128,\n          \"input_tokens\": 32983,\n          \"output_tokens\": 87,\n          \"cached\": 0,\n          \"input\": 32983,\n          \"duration_ms\": 15530,\n          \"tool_calls\": 0\n      }\n  }\n  ```\n\n  - This metadata can be used to present metadata to the user on STDERR when they are executing a non-interactive command.\n  - This metadata can be used to enhance the data we're providing to our logging platform\n\n  Your task is to:\n  \n  - research other examples online and fill in any other missing details not self-evident from the example data\n  - determine how best to feed the metadata to logging and non-interactive sessions.\n\n  The final output should be well formed, idiomatic Markdown. Links are Markdown links, Tables are Markdown tables. If you want to display a visual representation, using Mermaid code blocks are a good approach to this.\n\n  **IMPORTANT:**\n\n  - DO NOT CHANGE THE FRONTMATTER other than updating the `last_updated` property to today's date\n  - Write the content of your research into the body of this document, DO NOT create another document and have this document link to it!"
last_update: 2026-03-16
last_updated: 2026-03-16
---
The document is complete. Here's what was written:

**Strategy for Parsing OpenCode CLI JSON Output** — covering:

- **Corrected format identification** — OpenCode uses `--output-format json` (NDJSON), not `stream-json`. The original stub had Gemini CLI example data.
- **8 event types** documented with real NDJSON examples (`step_start`, `text`, `tool`, `step_finish`, `reasoning`, `retry`, etc.)
- **Key differences from other providers:**
  - **No aggregate result event** — usage is per-step, requiring accumulation
  - **Timestamps are Unix ms integers**, not ISO 8601
  - **Model not in stream** — must be carried from wrapper config
  - **Per-step cost** — unique among all providers
  - **Separate cache.read/cache.write** — more granular than others
- **Two event surfaces** — CLI NDJSON (~8 types) for real-time progress vs plugin bus (40+ types) for rich metadata, with a Mermaid diagram showing the architecture
- **Comparison table** across all 5 providers (Claude, Gemini, Codex, Kimi, OpenCode)
- **STDERR display strategy** — step summaries, tool activity in verbose mode, retry warnings
- **Logging pipeline integration** — token normalization mapping, synthetic session_end event, `OpenCodeStreamState` accumulator design
- **Implementation plan** with sequence diagram and output-mode rules