## Provider Event Support Analysis

| Provider | Hook Method | Tool Events Work? |
|----------|------------|-------------------|
| Claude | settings.json | ✓ Yes |
| Gemini | settings.json | ✓ Yes |
| OpenCode | plugins (fixed) | ✓ Now works |
| Codex | config.toml notify | Only turn_complete |
| Goose | None (stream-json) | ✗ NonHook |
| Kimi Code | None (wire mode) | ✗ NonHook |
| Qwen Code | None (stream-json) | ✗ NonHook |
| Roo Code | None | ✗ NonHook |

For NonHook providers (Goose, Kimi, Qwen, Roo), events are captured via:
- Stream JSON parsing (Goose, Qwen, Roo)
- Wire mode JSON-RPC proxy (Kimi)


