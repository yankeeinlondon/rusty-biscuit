# Stream Parsing

When **Claudine** is running in non-interactive mode it will ask the Agent to respond in a streaming format. Each Agent has slightly different structure and semantics but we standardize the way we want to interact with this streaming information with the [`StreamParser`](claudine/lib/src/stream/parser.rs) trait.


## Research

We have done deep research on the JSON streaming data that each of CLI Agents reports:

- [claude](claudine/docs/research/non-interactive-sessions/claude.md)
- [codex](claudine/docs/research/non-interactive-sessions/codex.md)
- [gemini](claudine/docs/research/non-interactive-sessions/gemini.md)
- [kimi](claudine/docs/research/non-interactive-sessions/kimi.md)
- [opencode](claudine/docs/research/non-interactive-sessions/opencode.md)
- [qwen](claudine/docs/research/non-interactive-sessions/qwen.md)
- [goose](claudine/docs/research/non-interactive-sessions/goose.md)
- [roo code](claudine/docs/research/non-interactive-sessions/roo-code.md)

## Streaming Schemas

Based on this research we've been able to establish the following schemas for the various providers:

### Claude

TODO

### Codex CLI

TODO

### Gemini CLI

TODO

### Kimi Code

TODO

### Opencode CLI

TODO

### Qwen CLI

TODO

### Goose

TODO

#### Roo Code

TODO



