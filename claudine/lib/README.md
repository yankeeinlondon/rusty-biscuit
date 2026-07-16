# Claudine Library

Core library for the Claudine cross-agent event handling, skill linking, and MCP management system. Provides the event model, provider adapters, dispatch pipeline, configuration management, agent capability catalog, MCP catalog/sync/runtime primitives, and skill synchronization logic used by the `claudine` CLI.

## Architecture

The library is organized into eighteen top-level modules plus the shared error type:

```
claudine/lib/src/
├── actions/        → Hook action types and response model
├── badges.rs       → Styled terminal badge constants (YOLO, Non-Interactive, Interactive, etc.)
├── composition/    → Markdown frontmatter composition (inline, direct, and sequence pipelines)
├── config/         → Agent detection and hook registration
├── dispatch/       → Event processing pipeline
├── events/         → Normalized 16-event lifecycle model (`AgenticEvent`, `EventMeta`, matrices)
├── harness/        → Timeouts, shell audit, attempt classification, and lifecycle recovery infrastructure
├── hook_adapters/  → Native hook request/response adapters (parse provider hook payloads; distinct from stream/providers)
├── linking/        → Cross-provider skill and command synchronization
├── messaging/      → Outbound messaging routes, resolution, and provider dispatch
├── mcp/            → MCP catalog, defaults, import/export, session, and injection
├── permissions/    → Provider-agnostic policy engine (queries, mutations, canonical model)
├── provider/       → Central provider catalog: `Provider` enum, `ProviderInfo`, behavior traits
├── reporting/      → JSONL-to-SQLite reporting index, sync, and typed queries
├── services/       → Cross-provider runtime policy services (Protect)
├── stream/         → Structured stream parsing for 6 providers + typed protocol models + summary/reporting
├── system_prompt/  → System prompt discovery, CLI switch resolution, and preparation
└── error.rs        → ClaudineError enum
```

### Actions (`actions`)

Types for hook actions that execute when events fire, and response types for blocking hooks:

- `HookAction` — 6-variant tagged enum: `SoundEffect`, `Speak`, `Bash`, `Call`, `Report`, `Message`
- `HookResponse` — Unified response a hook can return to influence agent behavior (decision, reason, updated input, additional context)
- `HookDecision` — 4-variant enum: `Allow`, `Deny`, `Ask`, `Continue`
- `LogTarget` — File (with daily rotation) or Server (HTTP POST with timeout)
- `ReportHandler` / `ReportFormat` — Report output formatting (Text, Json, Compact)
- `Mapper` / `CompiledMapper` — Transform command output into `HookResponse` (JsonField, JsonObject, ExitCode, Regex)

### Event Model (`events`)

16 normalized lifecycle events that abstract across all 7 provider APIs:

| Category | Events |
|----------|--------|
| Session | `session_start`, `session_end` |
| Prompt | `before_prompt` |
| Tool | `before_tool`, `after_tool`, `tool_error` |
| Turn | `turn_complete`, `turn_error` |
| Permission | `permission_request`, `human_in_the_loop` |
| Subagent | `subagent_start`, `subagent_stop` |
| Model | `before_model`, `after_model` |
| Other | `before_compact`, `notification` |

Key types:
- `AgenticEvent` — 16-variant enum with snake_case serde, descriptions, payload schemas, return schemas, and abbreviations
- `EventMeta` — Normalized event metadata (provider, event, tool name, error, prompt, session ID, timestamps, environment context)
- `ResolvedHook` — A fully resolved hook binding ready for execution (event, meta, provider, actions, can_block)
- `ClaudineConfig` — Canonical configuration type
- `EventBinding` — Event binding configuration
- `GlobalSettings` / `TtsSettings` / `LinkingSettings` / `CanonicalProviderSettings` — Settings types
- `EnvironmentContext` — Auto-detected OS, hardware, git, and repo context (via `sniff`)
- `event_support_matrix()` / `event_native_mapping_matrix()` — structured matrices that drive `claudine hooks --support` and `claudine hooks --mapping`. Both source every cell directly from `crate::provider::provider_info(provider).event_mapping`.

The `Provider` enum and `EventSupportLevel` type are owned by [`crate::provider`](#provider-catalog-provider) — the centralized providers refactor (Phase 8) removed the legacy `crate::events::Provider` re-export.

### Provider Catalog (`provider`)

Central catalog for the 7-variant `Provider` enum and all per-provider static facts. `provider_info(p) -> &'static ProviderInfo` is the single registry every per-domain dispatch flows through.

Key types:
- `Provider` — 7-variant enum (Claude, Codex, Gemini, Goose, KimiCode, OpenCode, QwenCode); serde snake_case identifiers remain `kimi_code` / `open_code` / `qwen_code`, while the catalog `slug` values are the canonical short forms `kimi` / `opencode` / `qwen`
- `PROVIDERS_DISPLAY_ORDER` — `[Provider; PROVIDER_COUNT]` array (7 entries) used by every matrix-style report
- `ProviderInfo` — full per-provider record: identity (`display_name`, `slug`, `binary`, `agent_offset`, `cli_aliases`, `docs_url`, `usage_dashboard_url`, `sniff_binding`), event mapping, four behavior trait objects, agent and resource accessors, and the Phase 5 typed catalog (`PathTemplate`, `OutputFormatSupport`, `EntrypointSpec`, `SystemPromptSpec`, `YoloSupport`, `ReasoningSupport`, `KnownGap`, `AcpSupport`, `PromptArgConventions`)
- `EventSupportLevel` — `Hook` | `NonHook` | `Acp` | `NotSupported` per provider-event pair
- `EventMappingTable` / `EventMapping` — per-provider event row table; the source of truth for support level, native names, parse aliases, and which rows participate in standard hook registration
- `ProviderBehavior`, `McpBehavior`, `AdapterBehavior`, `ConfiguratorBehavior` — focused behavior traits implemented as zero-sized provider structs and stored as `&'static dyn Trait` on `ProviderInfo`
- `AcpSupport`, `AcpServerMode`, `AcpEvent` — typed ACP capability descriptor surfaced by `claudine hooks --capture-method`

Adding an eighth provider is a matter of: adding the `Provider` variant, creating one `provider/<name>.rs` file with a `<NAME>_INFO: ProviderInfo` constant, registering it in `provider/registry.rs`, and adding a CLI-side `WrapperProfile` entry. See [`docs/topics/building-an-agent-wrapper.md`](../docs/topics/building-an-agent-wrapper.md) for the full checklist.

### Hook Adapters (`hook_adapters`)

Each provider has its own adapter implementing the `ProviderAdapter` trait. These
parse native hook request/response payloads — distinct from `stream/providers`,
which parse stdout NDJSON:

| Adapter | Parses | Status |
|---------|--------|--------|
| `claude` | `hook_event_name` field from settings.json hooks | Implemented |
| `codex` | JSONL stream fields + notify hook | Implemented |
| `gemini` | Settings.json hook events | Implemented |
| `opencode` | Plugin-based event names | Implemented |
| `goose` | Stream-json + env var (type/event field) | Implemented (non-blocking) |
| `kimicode` | Wire mode JSON-RPC (event_name/method field) | Implemented (blocking: tool, permission) |
| `qwen` | Stream-json output (event_name/type field) | Implemented (blocking: permission) |

The `adapter_for(provider)` factory returns the appropriate adapter singleton. Each adapter normalizes the provider's native JSON payload into `(AgenticEvent, EventMeta)` and can format `HookResponse` back into provider-native response payloads.

### Dispatch Pipeline (`dispatch`)

The core event processing pipeline runs in 6 steps:

1. **Select adapter** — `adapter_for(provider)`
2. **Parse event** — adapter normalizes raw JSON into `(AgenticEvent, EventMeta)`
3. **Load config** — merges user (`~/.claudine/config.json`) and repo (`.claudine/config.json`) configs, precompiling matcher and mapper regexes
4. **Look up binding** — finds `RuntimeEventBinding` for this provider + event, checks enabled and non-empty actions
5. **Check matcher** — precompiled regex match against event metadata (filters actions)
6. **Execute actions** — runs each action via `runner::execute_actions()`, collecting blocking responses from `Call` actions

Sub-modules:
- `loader` — Config file discovery, loading, merge logic, runtime compilation (matchers + mappers), and config save/validation
- `template` — `{{placeholder}}` Handlebars-style interpolation engine with 30 variables across 5 categories (legacy `{placeholder}` single-brace syntax is deprecated with warnings)
- `matcher` — Regex-based event filtering against tool name, notification type, or error
- `runner` — Executes actions (TTS via biscuit-speaks, logging, shell commands, sound effects via playa, report formatting)

**Config merge strategy**: repo-level provider configs completely replace user-level; global settings merge field-by-field with repo taking precedence. `linking.canonical_provider` merges slot-by-slot so user-scoped canonical providers survive when repo only sets repo-scoped slots.

### Template Variables (`dispatch::template`)

28 variables in 5 categories, available for `{{placeholder}}` interpolation in speak, report, and command templates:

| Category | Variables |
|----------|-----------|
| Event | `{{provider}}`, `{{event}}`, `{{timestamp}}`, `{{session_id}}`, `{{cwd}}`, `{{tool_name}}`, `{{error}}`, `{{prompt}}`, `{{agent_type}}`, `{{notification_type}}` |
| OS | `{{os.name}}`, `{{os.type}}`, `{{os.version}}`, `{{os.hostname}}` |
| Hardware | `{{hardware.arch}}`, `{{hardware.cpu}}`, `{{hardware.cores}}` |
| Git | `{{git.branch}}`, `{{git.is_dirty}}`, `{{git.head_sha}}`, `{{git.head_message}}`, `{{git.remote}}`, `{{git.hosting}}`, `{{git.repo_name}}`, `{{git.repo_org}}` |
| Project | `{{project.language}}`, `{{project.is_monorepo}}`, `{{project.monorepo_standard}}`, `{{project.monorepo_orchestrators}}`, `{{project.monorepo_tool}}` (deprecated alias) |

Shell environment variables are also supported via `{{env.VAR_NAME}}` with optional defaults: `{{env.MY_VAR || "fallback"}}`. The legacy single-pipe `|` form is no longer supported.

Unknown placeholders are left as-is. `None` values render as empty strings.

### Configuration (`config`)

Agent detection and hook registration for all 7 providers:

- `detect_agents()` — returns detected providers with their configurators
- `discover_agents_full()` — all 7 providers with install/registration status (`AgentInfo`)
- `get_configurator(provider)` — returns the configurator for a specific provider
- `AgentConfigurator` trait — `register()`, `deregister()`, `is_registered()`, `registered_events()`, `create_minimal_config()`, `supports_config_registration()`, `registerable_events()`, `is_cli_installed()`

Configurators handle each provider's config format:
- **Claude/Gemini**: JSON `settings.json` with hooks array
- **Codex**: TOML `config.toml` with notify section
- **OpenCode**: JSON `opencode.json` with plugins
- **Goose/KimiCode/Qwen**: Wrapper-only (no config-based registration)

Atomic file writes (`config::atomic`) prevent corruption during concurrent access. Config backup utilities (`config::backup`) preserve originals before modification.

### Skill Linking (`linking`)

Cross-provider resource synchronization via symlinks and format-converted derived artifacts. See [linking-strategy.md](../../.claude/skills/claudine/linking-strategy.md) for the full algorithm deep dive.

**Linkable resources** (4 types): Skill, Command, Agent, Script

**Support levels** per provider: Full, CustomFormat, Limited, None

**Sync strategies**:
- **Direct symlinks** for same-format providers (Markdown to Markdown)
- **Derived artifacts** for different-format providers (Markdown to TOML for Gemini commands, Markdown to YAML for Goose/KimiCode agents), with embedded hash markers for staleness detection

**Algorithm** (6 phases):
1. **Canonical selection** — elect one provider as the source of truth per `(scope, resource_type)` pair, preferring providers with existing valid assets
2. **Discovery** — scan provider directories for skills, commands, agents, and scripts
3. **Hashing** — xxHash each resource (recursive walk for skill directories, file content for single files)
4. **Conflict analysis** — classify resources as LinkCandidate, InSync, Conflict, or AlreadyLinked; also-reads-from providers are excluded from link targets to avoid redundant symlinks
5. **Compatibility classification** — parse canonical frontmatter, apply deterministic upgrades (alias duplication, name derivation), check required properties per target provider
6. **Apply** — create symlinks (absolute for user scope, relative for repo scope) or generate format-converted derived artifacts; never overwrites real directories

**Provider skill paths** (all 7 providers):

| Provider | User scope | Repo scope | Also reads from |
|----------|-----------|------------|-----------------|
| Claude | `~/.claude/skills/` | `.claude/skills/` | -- |
| Codex | `~/.codex/skills/` | `.codex/skills/` | `.claude/skills`, `.agents/skills` |
| Gemini | `~/.gemini/skills/` | `.gemini/skills/` | -- |
| Goose | `~/.config/goose/skills/` | `.goose/skills/` | `.claude/skills`, `.agents/skills` |
| KimiCode | `~/.config/agents/skills/` | `.kimi/skills/` | `.claude/skills`, `.agents/skills`, `.codex/skills` |
| OpenCode | `~/.config/opencode/skills/` | `.opencode/skills/` | `.claude/skills`, `.agents/skills` |
| QwenCode | `~/.qwen/skills/` | `.qwen/skills/` | -- |

Additional sub-modules: `canonical` (canonical provider selection), `capabilities` (provider resource support metadata with required/optional property schemas), `compatibility` (candidate/reference classification with alias duplication and name derivation), `conflict` (sync status analysis with also-reads-from awareness), `detector` (per-resource-type detectors via `LinkDetector` trait), `discovery` (legacy skill and command discovery), `execution` (analyze and apply resource links with derived artifact generation), `hashing` (xxHash content dedup with symlink resolution), `model` (data types including `ResourceReference` 9-variant state machine), `paths` (provider path resolution with repo-root awareness), `report` (link report types), `symlink` (symlink creation with relative path support).

### MCP (`mcp`)

Provider-agnostic MCP storage and provider-specific import/export/runtime integration:

- `catalog` - normalized server storage plus 5-tier ID/alias/query resolution
- `defaults` - user defaults (`~/.claudine/mcp/defaults.json`) and repo defaults (`<repo>/.claudine/mcp.json`), where repo replaces user
- `state` - provenance and managed ownership in `~/.claudine/mcp/provider-state.json`
- `import` - scans Claude, Codex, Gemini, and OpenCode native configs into the catalog with fingerprint dedupe
- `export` - dry-run/apply sync back to native configs with backups and managed-entry tracking
- `session` - computes runtime server sets from defaults, explicit `--use`, and non-interactive prompt `#tags`
- `inject` - runtime injection for OpenCode (env var) and Codex/Gemini (shadow-home config files)

Current runtime injection is intentionally narrower than import/export: Claude, Goose, Kimi, and Qwen do not have injectors yet. See [mcp-support.md](../docs/mcp-support.md) for the exact CLI-facing behavior and limits.

### Stream Parsing (`stream`)

Provider-native structured stream parsing for wrapped non-interactive sessions. Each provider's structured output (stream-json, JSONL, or NDJSON) is parsed live, extracting clean assistant text for stdout and metadata for stderr summaries and JSONL reporting.

**Provider parsers** (6):

| Parser | Format | Summary source |
|--------|--------|----------------|
| `claude` | stream-json | `result` event with duration, usage, cost, turns |
| `codex` | JSONL (`exec --json`) | `turn.completed` usage + `--output-last-message` file for text |
| `gemini` | stream-json | `result.stats` with token counts |
| `kimi` | stream-json | Latest `StatusUpdate` snapshot (no aggregate result) |
| `opencode` | NDJSON (`json`) | Accumulated per-step usage/cost |
| `qwen` | stream-json | Final result/usage event |

**Typed protocol models** (`stream::protocol`):

Each of the 6 supported providers has a serde-derived event model in `stream/protocol/<provider>.rs`. Every module exports a tagged `*Event` enum (`#[serde(tag = "type")]`) plus one struct per variant payload. Shared design rules:

- **Every field is optional** with `#[serde(default)]` so provider format evolution never breaks deserialization. There is no `#[serde(deny_unknown_fields)]` anywhere in `protocol/`.
- **No unknown-variant fallback** — when a provider emits an event whose `type` string isn't listed in the enum, `serde_json::from_value::<*Event>` returns `Err(_)` and the parser silently skips the line, matching the legacy `_ => Ok(None)` arm.
- **Helper methods carry alias resolution** — instead of exposing every field alias to handlers, each struct provides `resolved_*` / `take_*` helpers (e.g. `resolved_tool_name()`, `take_input()`) that walk all accepted aliases in a single place.

Per-provider idioms:

- **Claude** — `ClaudeEvent` has separate `Init` and `System` variants that both wrap `ClaudeInit`, funneling into the same handler. `ClaudeResult::effective_cost_usd()` picks `total_cost_usd` over the legacy `cost_usd`.
- **Codex** — Dotted event names work cleanly with `#[serde(rename = "thread.created")]`. `CodexItem` is a single flat struct covering every item subtype, with `is_tool_item_kind()` / `is_permission_item_kind()` and a typed `merge_started()` operation. `turn.started` uses an empty `CodexTurnStarted {}` struct because internally-tagged unit variants in serde have quirky behavior around extra fields.
- **Gemini** — `GeminiMessage.content` stays `Option<Value>` because Gemini emits content as either a plain string or an array of `{text: ...}` parts; the handler branches on `as_str()` vs `as_array()` after typed deserialization.
- **OpenCode** — The most complex parser. Tool fields can appear at the top level of an event OR nested inside a `part` object; `OpenCodeTool` captures both via `#[serde(flatten)]` plus a separate `part: Option<OpenCodeToolFields>`, and `OpenCodeTool::resolve()` collapses both locations into a `ResolvedOpenCodeTool`. `OpenCodeStepStart` uses `#[serde(rename = "sessionID")]` for the camelCase session ID.
- **Qwen** — The `system` event is dispatched only when `subtype == "session_start"` via `QwenSystem::is_session_start()` + `into_init()`. `QwenTool::take_input()` accepts five aliases: `input`, `parameters`, `arguments`, `args`, `params`.
- **Kimi** — `KimiContent::resolved_text()` implements a three-way fallback (`content` array → top-level `text` → `content` as string). `KimiStatusUpdate::resolved_context()` returns a `KimiContextUsage` whose `computed_percent()` falls back to computing `used/total * 100.0` when the provider doesn't pre-supply `percent`.

**Two-pass `feed_line` dispatch**: parsers parse the raw line into a `serde_json::Value` first (preserves the malformed-line error path and keeps a raw copy available for `raw_summary` construction in result events), then attempt typed deserialization into the provider-specific `*Event` enum. Every protocol module has a `#[cfg(test)] mod tests` block covering each event variant, the major field aliases, and the `unknown_event_type_fails_typed` contract — those tests are the safety net for provider format drift.

**Infrastructure**:
- `parser` — `StreamParser` trait and `StreamEventSink` callback interface for coarse event handling (session start, turn lifecycle, tool events)
- `protocol` — Strongly typed serde-derived event models, one module per provider, plus a shared `ProtocolError` type
- `summary` — `StreamExecutionSummary` struct: provider-agnostic metadata (session ID, model, tokens, cost, duration, tool calls, rate limits, context usage)
- `token_usage` — `NormalizedTokenUsage` with input/output/total/cache_read fields
- `stderr` — Verbosity-aware stderr formatting (start summary, completion summary, compact line for `--quiet`)
- `reporting` — Converts `StreamExecutionSummary` to `EventMeta` for synthetic JSONL summary events

**Execution modes** (in CLI `wrap/exec.rs`):
- `run_child_stream()` — live parsing with assistant text piped to terminal
- `run_child_stream_capture()` — parsing with captured text for composition flows

### Composition (`composition`)

Markdown frontmatter-based composition pipelines for delivering prompts to provider sessions. Three canonical modes:

- **Direct composition** (`claudine compose <file>`): composes the full document as a prompt without mutating the source file
- **Inline composition** (`claudine inline-compose <file>`): reads frontmatter `prompt` as input, then rewrites the document body from the provider's returned content while preserving source frontmatter
- **Sequence composition** (`claudine sequence <file>`): runs a serial sequence of composition steps from a single document, reusing wrapper-grade execution with a shared approval cache and `FAIL_FAST` propagation across steps

Sub-modules:
- `resolve` — source resolution via `biscuit-file::FileReference` with read/write permission validation
- `prepare` — builds a `PreparedComposition` (effective frontmatter, composed body, pre-execution hashes) via `prepare_direct()` / `prepare_inline()` with `PrepareOptions`
- `select` — deterministic provider selection (explicit flag → single-installed → frontmatter hint → config favorite → interactive chooser)
- `preflight` — shell approval collection and execution for source `::shell` directives and lifecycle stack `shell` actions
- `closure` — inline closure plan that merges provider-returned content back into the source file atomically (preserves frontmatter, updates `last_updated`)
- `sequence` — sequence plan parser, normalizer, and per-step overlay builder for `claudine sequence`
- `lifecycle` — `LifecycleEmitter` trait and `LifecycleRunGuard` RAII guard that emit the seven composition lifecycle signals (`initialize`/`start`/`success`/`blocked`/`failure`/`finalize`/`loop`) to external observers; includes `DefaultLifecycleEmitter`, the per-event stack model, and the static lifecycle validators
- `guardrails` — inline composition guardrails appended to prompts to constrain output shape
- `types` — shared types including `PreparedComposition`, `SelectedProvider`, `SequencePlan`, `SequenceStep`, `SharedApprovalCache`, `CompositionMode`, and `SystemPromptInput`

Execution parity note (2026-06-16): the CLI-side execution of `compose` and `inline-compose` flows through a single `run_harness_loop` path with `HarnessPromptMode::Compose` or `HarnessPromptMode::Inline`. Bare documents (no harness frontmatter) yield the empty plan. The loop handles structured streaming, captured/non-structured fallback, inline closure, summary emission, and lifecycle-driven recovery in one place.

### Badges (`badges`)

Styled terminal badge constants for the execution line header: `YOLO`, `NON_INTERACTIVE`, `INTERACTIVE`, `VERBOSE`, `COMPOSE`, `INLINE_COMPOSE`, `REPO_FLAG`, and scope badges (`USER_SCOPED`, `REPO_SCOPED`, `MASKED_REPO_SCOPED`).

### Services (`services`)

Cross-provider runtime policy engines that operate on normalized event context:

- `protect` — Capability-aware policy evaluation service used to normalize safety decisions (`allow`, `ask`, `stop`, `advisory`) across providers with different control surfaces.

### Permissions (`permissions`)

Provider-agnostic permission policy engine. `PolicyEngine` is Claudine's canonical source of truth for provider permission state: it loads provider-native config, composes it with CLI/runtime overrides, normalizes the result into a canonical cross-provider model, answers structured permission queries with explanation and provenance, and plans permission mutations.

Sub-modules:
- `engine` — `PolicyEngine` and `ProviderPolicyHandle` for per-provider operations
- `backend` — `ProviderPolicyBackend` trait plus `BackendCapabilities` and `BackendFidelity` describing what each provider can express natively
- `canonical` — canonical cross-provider model: `CanonicalPolicy`, `CanonicalApprovalMode`, `CanonicalSandboxMode`, `NetworkPolicy`, `FilesystemPolicy`, `McpAccessPolicy`, `SubagentRule`, `CommandAccessRule`, `PathAccessRule`, `DomainAccessRule`, rule provenance, and fidelity flags
- `context` — `PolicyContext`, `CliPolicyInput`, and `ProjectTrustContext` carrying CLI overrides and trust signals
- `native` — `NativeEffectivePolicy`, `NativePolicyLayer`, `PolicySource`, and `ProviderCliOverrides` describing the raw provider-native layering
- `query` / `explain` / `change` / `mutation` — structured queries, provenance-aware explanations, and mutation planning
- `matchers` — shared glob/regex matching primitives reused across providers
- `providers` — per-provider backend implementations

The engine is independent from `ProtectService`. Protect remains a runtime decision layer that may consume `PolicyEngine` in future revisions.

### Messaging (`messaging`)

Outbound messaging routes for the `Message` hook action. Supports Discord, Slack, Signal, and WhatsApp providers:

- `config` — `MessagingRouteConfig` and `ScopedMessagingSettings` for route definitions at user and repo scope
- `resolve` — `ResolvedMessagingRoute` plus secret/image/recipient resolution (`resolve_secret`, `resolve_image_path`, `parse_signal_recipient`) and effective route selection across scopes
- `send` — `execute_message` and `execute_resolved_message` for dispatching markdown messages through the chosen provider, with Discord image attachment support

### System Prompt (`system_prompt`)

Discovery, CLI switch resolution, and preparation of system prompts injected by the wrapper:

- Discovers standard `system-prompt.md` files across package, package-area, repo, user, and current-directory scopes (`StandardPromptScope`)
- Resolves explicit CLI switches (`--system-prompt` / `--append-system-prompt` / `--replace-system-prompt`) via `SystemPromptArgs` and `SystemPromptMode`
- Captures provenance via `SystemPromptSource` (`StandardDiscovered` or `ExplicitFile`)
- `resolve_and_prepare()` and `prepare_system_prompt()` build the final system prompt string passed to the provider; `LaunchContext` carries the resolved state into wrapper execution

### Reporting (`reporting`)

Library-first reporting over Claudine's JSONL event logs:

- `ReportingStore` — opens `~/.claudine/logs/metrics.db`, creates the schema, syncs JSONL logs, and exposes typed query methods
- `paths` — shared log/db path resolution so log writing and reporting use the same filesystem layout
- `ingest` — incremental JSONL ingestion keyed by `(source_file, source_offset)` with conservative session fallback rules
- `queries` — typed daily summary, sessions, tools, errors, repos, and trends queries
- `metrics` — derived metrics such as autonomy ratio, research-vs-action ratio, recovery rate, and context pressure
- `types` — stable result models used by both terminal rendering and `--json` output

### Harness (`harness`)

Timeouts, shell policy, and runtime attempt classification for composed prompt pipelines:

- `model` — core data types: `HarnessPlan`, `ProcessTermination`, `AttemptOutcome`, `FailureEvent`, `GuardContext`, and shell-audit reporting types (`AuditedCommand`, `ShellAuditReport`)
- `error` — `HarnessError` enum covering parse, runtime, shell, shell-audit, and path resolution failures
- `parse` — frontmatter-to-plan parser accepting `timeout`, `step_timeout`, `timeout_warn`, and `step_timeout_warn`
- `audit` — shell command collection across lifecycle stacks for pre-flight approval
- `shell` — shell policy adapter reusing Darkmatter's tokenizer, blacklist/whitelist, and approval handler infrastructure
- `resolve` — source-relative path resolution (`@repo/path`, `./local`, `/absolute`)
- `timeout` — human-friendly duration parser (`30s`, `5m`, `2h`)
- `runtime` — `build_attempt_outcome()` and `classify_failure()` for mapping stream summaries to harness outcomes
- `report` — stderr status emitters used by the harness loop and lifecycle recovery routing
- `speech` — best-effort TTS helpers for runtime lifecycle side effects

## Action Execution

| Action | Behavior | Blocking |
|--------|----------|----------|
| `Speak` | TTS via biscuit-speaks with template interpolation | Fire-and-forget (tokio::spawn) |
| `SoundEffect` | Playa embedded effects with volume/speed control | Fire-and-forget (tokio::spawn_blocking) |
| `Message` | Send Markdown notifications through the configured messaging route, with Discord image attachment support in v1 | Fire-and-forget (tokio::spawn) |
| `Log` (file) | Append JSONL, creates parent dirs, supports daily rotation | Synchronous |
| `Log` (server) | POST JSON with configurable timeout and headers | Non-fatal on failure |
| `Report` | Write to stdout with optional template/format (Text, Json, Compact) | Synchronous |
| `FireAndForget` | Execute command asynchronously without waiting | Fire-and-forget (tokio::spawn) |
| `Call` | Execute command synchronously with timeout and response mapper | Blocking (returns `HookResponse`) |

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `sniff` | Environment detection (OS, hardware, git, repo) |
| `biscuit-hash` | xxHash for skill content deduplication |
| `playa` | Sound effect playback (async) |
| `biscuit-speaks` | TTS for speak actions |
| `serde` / `serde_json` | JSON serialization for configs and events |
| `tokio` | Async runtime for concurrent action execution |
| `regex` | Event matcher and mapper pattern compilation |
| `reqwest` | HTTP client for log server POSTing |
| `rusqlite` | SQLite-backed reporting index and query layer |
| `toml_edit` | Format-preserving TOML edits (Codex config) |
| `thiserror` | Error type derivation |
| `walkdir` | Directory traversal for skill discovery |
| `chrono` | Timestamp handling and daily log rotation |
| `dirs` | Home directory resolution |
| `serde_yaml` | YAML parsing for skill frontmatter |
| `url` | URL parsing and validation |

## Lessons Learned

- **Hook handlers must respond fast**: Providers in non-interactive mode (`--print`, `--prompt`) may cancel hooks that don't produce stdout output within their shutdown window. `claudine handle` enforces a hard **5-second execution deadline** by default to prevent blocking the parent agent session. Individual `Bash` and `Message` actions also have tighter 3s timeouts when running inside a hook handler. Non-blocking events return a `{}` JSON acknowledgment via the adapter's `non_blocking_ack()` method — silent stdout is interpreted as "hook cancelled" by Claude Code, Gemini, and others.
- **Refined structured output**: Wrapped non-interactive sessions follow a **9-section model** (execution line, env, system prompt, agent prompt, session ID, thinking prose, tool/info events, final STDOUT, and metadata) with strictly enforced spacing. Thinking prose is rendered as a `BlockQuote` on stderr. Tool calls use a canonical `ToolCallDisplay` contract (`🔧 →` / `🔧 ←`) with humanized names and summarized inputs/results, managed by `LiveSemanticSink`.
- **Config merge is intentionally asymmetric**: repo provider configs fully replace user-level (not merged per-event) to give projects complete control. Settings merge field-by-field because they're global preferences. Nested structs like `linking` and `canonical_provider` also merge field-by-field — repo non-`None` values override user, but user-only fields (e.g. `user_skill`) survive when the repo config doesn't set them.
- **All 7 adapters are implemented**: each provider adapter has full event mapping, metadata extraction, and tests. Claude, Gemini, OpenCode, and Codex use config-based hooks; Goose, KimiCode, and Qwen parse stream-json or wire-mode payloads directly. KimiCode and Qwen support blocking responses; Goose is observation-only.
- **Template regex is lazy-compiled**: `LazyLock<Regex>` ensures the Handlebars `\{\{\s*([^{}]+?)\s*\}\}` pattern compiles once across all interpolation calls.
- **Sound effects are fire-and-forget**: TTS and sound playback spawn tokio tasks to avoid blocking the event pipeline. Log and report actions run inline because they're fast.
- **Atomic writes prevent config corruption**: all config file mutations go through `config::atomic` to handle concurrent hook firings safely.
- **Runtime config precompiles regexes**: matcher patterns and Call action mapper regexes are compiled once at config load time, failing fast on invalid patterns with contextual error messages.
- **Legacy single-brace templates are deprecated**: `{placeholder}` is automatically rewritten to `{{placeholder}}` with a tracing warning. New configs should use Handlebars-style double braces.

## Compatibility Notes

### Contextual Errors (2026-04)

Several error variants were refactored to carry typed `darkmatter::markdown::MarkdownError` values instead of pre-stringified text so the CLI's cause-chain walker can render rich `BlockError` reports (path, line, hint, transclusion chain, etc.).

- `ClaudineError::SystemPromptComposition(String)` → `SystemPromptComposition(#[from] MarkdownError)`
- `composition::CompositionError::ComposeFailed(String)` → `ComposeFailed(#[source] MarkdownError)`
- `composition::CompositionError::PreFlightDiscoveryFailed(String)` → `PreFlightDiscoveryFailed(#[source] MarkdownError)`
- `composition::CompositionError::LifecycleInvalid { property, message }` — new variant that replaces the previous `ComposeFailed(format!("invalid …"))` route for frontmatter lifecycle deserialization failures.

External consumers that pattern-match on these variants or inspect their string payload need to update their matches. The `Display` output for each variant still begins with the same human-readable prefix (`"system prompt composition failed: …"`, `"compose failed: …"`, `"pre-flight discovery failed: …"`), so consumers that only format-and-log these errors are unaffected.

The harness sites at `harness/parse.rs` (`tokenize()`) and `harness/audit.rs` (`parse_directives()`) remain deliberately out of scope; they discard darkmatter error detail on internal paths that are not user-facing today and are tracked for a follow-up.
