# Supported Platforms

Claudine supports 7 agentic CLI platforms. Each platform has varying levels of support for skills, slash commands, sub-agents, and hook events. Run `claudine providers` to see the live capability matrix:

```
┌───────────┬───────┬───────┬───────┬───────┐
│ Provider  │ Skill │ Slash │ Agent │ Hooks │
├───────────┼───────┼───────┼───────┼───────┤
│ Claude    │  ✅   │  ✅   │  ✅   │   13  │
│ Codex     │  ✅   │  ✅   │  ✅   │    1  │
│ Gemini    │  ✅   │  ✅   │  ✅   │   10  │
│ Goose     │  ✅   │  ✅   │  ✅   │    0  │
│ Kimi Code │  ✅   │  ❌   │  ✅   │    0  │
│ OpenCode  │  ✅   │  ✅   │  ✅   │   13  │
│ Qwen Code │  ✅   │  ✅   │  ✅   │    0  │
└───────────┴───────┴───────┴───────┴───────┘
```

## Column Definitions

- **Skill** -- whether the provider can consume markdown skill files via symlink or native directory
- **Slash** -- whether the provider supports custom slash commands
- **Agent** -- whether the provider supports sub-agent delegation
- **Hooks** -- number of distinct lifecycle events the provider exposes as hookable events
