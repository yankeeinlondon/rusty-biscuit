# Claudine Library Module Map

Detail behind [SKILL.md](SKILL.md).

The primary public modules are below; the shared `error` type and flat
`provider_id` leaf sit beside them. Full detail is in
[architecture.md](architecture.md).

| Module | Responsibility |
|--------|----------------|
| `actions` | Hook action types and responses |
| `badges` | Styled terminal badge constants |
| `composition` | Markdown frontmatter composition (direct/inline/sequence) plus the loop engine |
| `config` | Agent detection, hook registration, atomic writes, backups |
| `diagnostics` | Typed diagnostic facets, discovery, effective selection, and snapshots |
| `dispatch` | Event processing pipeline, templates, matchers, expression bridge |
| `events` | The normalized 16-event lifecycle model |
| `harness` | Shell audit, timeouts, runtime attempt classification, speech helpers, and kept lifecycle recovery infrastructure (no validation/handler DSL — see [Validations and Handlers → Lifecycle Stacks](validations-and-handlers.md)) |
| `hook_adapters` | Native hook request/response adapters (parse provider hook payloads; distinct from `stream/providers` stdout NDJSON parsers) |
| `interrupt` | Process-scoped user-interrupt state shared by lifecycle work |
| `linking` | Cross-provider resource sync with portability classification |
| `mcp` | Catalog, defaults, provider-state, import/export, runtime injectors |
| `messaging` | Outbound routes (Discord/Slack/Signal/WhatsApp); desktop notifications are separate and zero-config |
| `model_catalog` | Model validation against the generated expected-offerings baseline (+ user overrides), `family_latest` alias resolution, and the dynamic-listing drift channel |
| `opencode_config` | OpenCode configuration parsing and projection |
| `permissions` | Provider-agnostic policy engine (`PolicyEngine`) |
| `protect` | Standalone regex deny catalog (bash commands, write/edit paths, MCP responses) |
| `provider` | Generated provider metadata registry plus hand-written behavior |
| `render` | Functional render components — `FinalMessage`, `AgentPrompt`/`SystemPrompt` (under `render/prompt/`, absorbed the former `prompt_reporting` module), `EventRenderer` + the exhaustive `DISPATCH` table (live-sink stderr status dispatch), the dual-target `MetricsReport` (`TerminalRenderable` + `BrowserRenderable`), and the `StreamRenderable` span contract (`open`/`append`/`flush_idle`/`close`) with its `AssistantStream` streaming-markdown component, and `TaskStream`/`TaskStreamFrame` + the two-channel `TaskStreamSink` seam and its `TaskLiveOutput` binding (attributed color-bar framing for group tasks — headers/footers on the status channel, task body data on the data channel; `TaskBar::Invisible` gives serial work the same geometry); all consume data + policy (`DisplayPolicy`), never `match provider` |
| `reporting` | JSONL-to-SQLite metrics index |
| `runaway` | Pure content-guard detector (exit-expressions, group-cycle repetition, volume cap) + per-layer config; trips map to `ProcessTermination::Aborted` |
| `signals` | Generated and bespoke normalized signal catalog and hub |
| `stream` | Structured stream parsing through 8 parser implementations for 9 provider identities (typed models in `stream::protocol`; Kilo reuses OpenCode's parser; Goose has no native stream parser) |
| `system_prompt` | Launch-CWD detection (`LaunchContext`), `system-prompt.md` discovery, `ResolvedSystemPrompt` resolution |
| `provider_id` *(leaf)* | `Provider` enum, `provider_info()`, `PROVIDERS_DISPLAY_ORDER`, `OutputFormatSelector` (split from `provider/mod.rs` to break the `provider` ⇄ `stream` import cycle) |
