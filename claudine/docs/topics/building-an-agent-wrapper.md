---
prompt: |-
    The claudine package wraps a number of popular agentic CLI providers including Claude Code, Codex, Qwen CLI, Kimi 
    CLI, Gemini CLI, Goose, OpenCode, and Roo Code. 
    
    In adding support for these platforms we had to a good amount of research but hopefully all of the key salient
    information on each platform is now metadata in the code base. 

    Your job is to build out the body of this document to act as a comprehensive guide to building support for any new
    providers we decide to add. This document should be broken up into the following sections:

    - `## Metadata`
        - provides details about how the source code records metadata about each provider
            - Important URLs
            - Skill, Agent Definition, and Slash commands locations (user and repo scoped) and format (Markdown, YAML, TOML, etc.)
            - Format of non-interactive session streaming
            - Events support and mapping of native event names to Claudine event names
            - Information about logging (files, db) location and format
            - ACP support
            - CLI parameter mapping
            - and all other key metadata
        - if there are gaps in metadata today which we believe could benefit us going forward include that in a subsection `### Future Improvements to Metadata`
    - `## Checklist`
        - create a checklist of things which will be necessary to do when adding a new provider to Claudine
    - `## Things to Look Out For`
        - describe lessons learned or things which can be complicated when implementing a new provider
last_updated: 2026-04-26
---
I have enough context. Let me write the document body now.

```markdown
# Building an Agent Wrapper

Claudine supports eight agentic CLI providers today, and the architecture is deliberately built so that a ninth provider should mostly be a matter of populating typed metadata and implementing a small number of focused traits. This document walks through where each piece of provider-specific information lives in the codebase, what gaps still exist, and the practical steps and pitfalls of adding a new provider.

## Metadata

Provider knowledge is intentionally fragmented across several modules so that each concern (hooks, streaming, linking, wrapping, MCP, model catalog, logging) can evolve independently. The flip side of that decision is that "metadata for provider X" is not a single struct — it is a coordinated set of definitions that all reference the same `Provider` enum variant.

### `Provider` enum — the canonical identifier

The `Provider` enum in [`claudine/lib/src/events/provider.rs`](../../lib/src/events/provider.rs) is the canonical identifier used everywhere else. It is the *minimum* surface a new provider must implement; nothing else compiles until this entry exists.

Per-variant data attached directly to `Provider`:

- `cli_aliases()` — short names accepted by `--provider`, fuzzy match, and the argv pre-parser.
- `parse_cli_name()` / `fuzzy_match_cli_name()` / `fuzzy_match_all()` — three-tier (exact / prefix / contains) input resolution shared with the `init` wizard, the wrap subcommands, and the composition picker.
- `sniff_ai_cli()` — bridge to the `sniff` crate's install detector (`InstalledAiClients`). The `sniff` enum is the source of truth for "is this binary on PATH"; if the variant is missing there it must be added in `sniff` first.
- `detect_from_payload()` — payload-shape heuristic used when an inbound hook event lacks an explicit provider hint.
- `as_slug()` — snake_case identifier used for filenames, JSON keys, and config sections.
- `agent_offset()` — relative directory used when constructing shadow `HOME` overrides for repo-scoped MCP injection (e.g. `.claude`, `.codex`, `.opencode`).
- `docs_url()` and `usage_dashboard_url()` — strings consumed by status badges and stream summaries.
- `supports_skills()` — quick boolean used by the linker.

Display order is fixed in `PROVIDERS_DISPLAY_ORDER` so matrix-style reporting is deterministic.

### Agent capability catalog

The richer descriptive metadata lives under [`claudine/lib/src/agents/`](../../lib/src/agents/). Each provider gets one file with a `*_capabilities()` function returning an `AgentCapabilities` struct (see the model in [`agents/model.rs`](../../lib/src/agents/model.rs)). Existing examples: [`claude_code.rs`](../../lib/src/agents/claude_code.rs), [`codex.rs`](../../lib/src/agents/codex.rs), [`gemini_cli.rs`](../../lib/src/agents/gemini_cli.rs), [`goose.rs`](../../lib/src/agents/goose.rs), [`kimi_code.rs`](../../lib/src/agents/kimi_code.rs), [`opencode.rs`](../../lib/src/agents/opencode.rs), [`qwen_cli.rs`](../../lib/src/agents/qwen_cli.rs), [`roo_code.rs`](../../lib/src/agents/roo_code.rs).

`AgentCapabilities` aggregates:

- `AgentMeta` — `id`, `display_name`, `binary`.
- `AgentDocs` — homepage and per-feature documentation URLs (skills, slash, subagents, scripts).
- `ConfigCapabilities` — `user_files`, `project_files`, `local_files`, plus a `ConfigFormat` enum (`Json`, `Jsonc`, `Toml`, `Yaml`, `Mixed`).
- `RuntimeCapabilities` — bundles seven sub-structs:

    - `ModelCapabilities` (CLI flags, `/`-commands, aliases, precedence, notes)
    - `NonInteractiveCapabilities` (entrypoints, stdin, output formats, structured output, resume, limitations)
    - `SystemPromptCapabilities` (supplement sources, replacement mechanisms, memory files)
    - `PermissionCapabilities` (modes, yolo flag, sandbox modes, allow/denylist controls)
    - `ReasoningCapabilities` (`ReasoningStyle::{NamedLevels, NumericBudget, BinaryToggle, ProviderSpecific, NotDocumented}`)
    - `LoggingCapabilities` (session and log locations, debug and telemetry controls)
    - `BillingCapabilities` (`BillingModel::{Subscription, PerToken, PrepaidCredits, ProviderOnly}` plus notes)

- `SkillsCapabilities`, `SlashCommandCapabilities`, `SubagentCapabilities`, `ScriptCapabilities` — each carries a `CapabilityStatus`, a `PathDiscovery` (user / project / admin / extension paths plus precedence rules), an optional `FrontmatterContract`, and the activation/invocation enums.
- `ConfidenceProfile` — a coarse `Confidence::{High, Medium, Low}` per area plus a `gaps: Vec<&'static str>` of explicit research debt.

A static descriptor is registered through [`agents/registry.rs`](../../lib/src/agents/registry.rs) (`agent_for(AgentId)`, `all_agents()`, `parse_agent_id()`), and the descriptor file is the single source of truth surfaced by `claudine providers`, `claudine hooks --variables`, and the `init` wizard.

### Hook events and native-name mappings

Event metadata lives in [`claudine/lib/src/events/`](../../lib/src/events/). The 16-variant `AgenticEvent` enum in [`events/agentic_event.rs`](../../lib/src/events/agentic_event.rs) is the unified lifecycle vocabulary. New providers do not extend this enum; they project their native events onto it.

Two pieces of per-provider event metadata are required:

- **Support level** — `Provider::event_support_level()` returns `EventSupportLevel::{Hook, NonHook, NotSupported}` for each `AgenticEvent`. `Hook` means "registerable via config-file modification"; `NonHook` means "captured via wrapper, wire-mode proxy, or stream parsing"; `NotSupported` is unreachable. The matrix is keyed off the `Provider`/`AgenticEvent` pair in a single `match`.
- **Native names** — `Provider::native_event_name()` returns the provider's own string identifier for each event. For providers that share registration logic with parsing (currently Claude, Gemini, OpenCode), this is sourced from a `SharedNativeEventMapping` table (`CLAUDE_SHARED_NATIVE_MAPPINGS`, `GEMINI_SHARED_NATIVE_MAPPINGS`, `OPENCODE_SHARED_NATIVE_MAPPINGS`) so registration and adapter parsing cannot drift apart. Other providers fall through to the per-variant match in `native_event_name()`.

`event_support_matrix()` and `event_native_mapping_matrix()` in [`events/matrix.rs`](../../lib/src/events/matrix.rs) build the structured tables that back `claudine hooks --support` and `claudine hooks --mapping`.

### Hook adapters and configurators

Two trait surfaces complete the hook story:

- **Adapters** in [`claudine/lib/src/adapters/`](../../lib/src/adapters/) (one file per provider) take a raw inbound payload, identify the native event name, and produce an `AgenticEvent` with normalized fields. `Provider::detect_from_payload()` is the dispatch primitive when the provider is not declared up front.
- **Configurators** in [`claudine/lib/src/config/`](../../lib/src/config/) implement `AgentConfigurator` from [`config/trait_def.rs`](../../lib/src/config/trait_def.rs). They handle `register`, `deregister`, `is_registered`, `registered_events`, `create_minimal_config`, and `registerable_events`. Providers that have no native hook surface set `supports_config_registration()` to `false` and rely on wrapper or stream-parsing capture instead.

### Stream parsing and structured output

Each provider that emits a structured output stream has three coordinated artifacts:

- A `StreamProtocol` mapping in [`stream/mod.rs`](../../lib/src/stream/mod.rs): `stream_protocol_for(provider)` returns one of `StreamJson`, `Ndjson`, or `Jsonl` (or `None` for providers without structured output).
- A typed protocol module under [`stream/protocol/`](../../lib/src/stream/protocol/) — one file per provider (`claude.rs`, `codex.rs`, `gemini.rs`, `kimi.rs`, `opencode.rs`, `qwen.rs`). Each defines a tagged `*Event` enum (`#[serde(tag = "type")]`) with one struct per variant payload, every field defaulted via `#[serde(default)]`, and helper methods that resolve aliasing (`resolved_tool_name()`, `take_input()`, `effective_cost_usd()`, etc.).
- A semantic parser (`*_semantic.rs` siblings of `protocol/`) that consumes the typed events and produces `SemanticEvent`s through `SemanticEventSink`. `create_semantic_parser()` in `stream/mod.rs` is the dispatch.

Unknown event types fall through silently (matching the legacy `_ => Ok(None)` behavior); there is intentionally no `#[serde(deny_unknown_fields)]` anywhere in `protocol/`. Each protocol module ships an `unknown_event_type_fails_typed` test that pins this contract.

### Wrapper profile (CLI parameter mapping)

Direct provider wrappers (`claudine claude`, `claudine codex`, …) live behind the `WrapperProfile` trait in [`claudine/cli/src/commands/wrap/profile.rs`](../../cli/src/commands/wrap/profile.rs). Each implementation is a static unit struct (`ClaudeWrapper`, `CodexWrapper`, etc.) registered with `profile_for_provider()`. Per-provider responsibilities the trait covers:

- `binary()` and `agent_env()` — the executable name and the value injected as the `AGENT` env var.
- `apply_yolo()` / `apply_yolo_for_mode()` / `has_supported_yolo()` / `reject_direct_yolo()` — Claudine canonicalizes auto-approval to `--yolo` and the wrapper translates it to `--dangerously-skip-permissions` (Claude), `--dangerously-bypass-approvals-and-sandbox` (Codex), `--yolo` (Gemini, Kimi, Qwen), or `--dangerously-skip-permissions` injected only in non-interactive mode (OpenCode).
- `apply_entrypoint()` / `apply_non_interactive_flags()` / `validate_non_interactive_requirements()` — inject the right subcommand (`exec`, `run`) and reject incompatible flags.
- `apply_model()` / `apply_output_format()` / `apply_system_prompt()` / `apply_sandbox()` — map the universal CLI surface to provider-specific flags, env vars, and temp artifacts.
- `prompt_delivery()` and `prompt_arg_conventions()` (`PromptArgConventions { prompt_flags, entrypoint, value_taking_flags }`) — describe how the prompt sits in argv so the pre-parser can locate and remove it cleanly.
- `stdout_noise_prefixes()` / `stderr_noise_prefixes()` / `suppress_structured_stderr_on_success()` — silence harmless provider chatter in non-interactive output.
- `supports_structured_stream()` / `stream_protocol()` / `apply_structured_stream()` — bridge the wrapper to the stream parser.
- `build_resume_args()` / `supports_resume()` / `supports_interactive_inline_closure()` — session resume and inline-compose closure support.
- `allowed_env_keys()` — env vars that bypass the sensitive-key sanitizer.

Roo Code is intentionally absent from this dispatch (`profile_for_provider(Provider::RooCode)` returns `None`) because it runs as a VS Code extension and has no standalone CLI binary.

### Linking metadata (cross-provider sync)

[`claudine/lib/src/linking/capabilities.rs`](../../lib/src/linking/capabilities.rs) defines `ProviderCapabilities`, parallel to the agent catalog but focused on cross-provider portability. Each `LinkableResource` (`Skill`, `Command`, `Agent`, `Script`) gets a `ResourceSupport` carrying:

- `SupportLevel::{Full, CustomFormat, Limited, None}` and a `ResourceFormat::{Markdown, Toml, Yaml, Mcp, BuiltinOnly, Executable}`.
- `repo_path` / `user_path` — relative to repo root and `$HOME` respectively.
- `also_reads_from` — additional directories the provider auto-discovers (this is how "OpenCode reads `.claude/skills`" is encoded).
- `properties: Option<ResourcePropertySchema>` — required and optional frontmatter/config fields, plus a `source_doc` reference back into `claudine/docs/cross-referencing/<provider>.md`.

`SkillFrontmatter` is a per-flag boolean record (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed_tools`, `user_invocable`, `disable_model_invocation`) used by the linker to decide whether a skill is portable to a target provider.

### MCP support metadata

MCP capability is split between three surfaces:

- The normalized catalog itself is provider-agnostic and lives under `~/.claudine/mcp/` (`catalog.json`, `defaults.json`, `provider-state.json`).
- Per-provider import/sync logic lives in [`claudine/lib/src/mcp/import.rs`](../../lib/src/mcp/import.rs) and [`mcp/state.rs`](../../lib/src/mcp/state.rs) — only the providers wired in actually appear (Claude, Codex, Gemini, OpenCode, Roo at the time of writing).
- Per-provider runtime injection logic lives in [`mcp/inject.rs`](../../lib/src/mcp/inject.rs); only Codex, Gemini, and OpenCode currently implement runtime injection. Other providers print a guidance message pointing at `claudine mcp export <provider> --apply`.

### Logging and reporting

Claudine's own logs are provider-agnostic and live under `~/.claudine/logs/` — daily-rotated JSONL files (`YYYY-MM-DD.jsonl`) and a SQLite metrics index at `metrics.db`. Path resolution is centralized in [`claudine/lib/src/reporting/paths.rs`](../../lib/src/reporting/paths.rs).

The provider's *own* logs are documented (not consumed) through `LoggingCapabilities` in the agent catalog — `session_locations`, `log_locations`, `debug_controls`, `telemetry_controls` are all `Vec<&'static str>` description fields. Examples encoded today:

| Provider | Sessions | Logs |
|----------|----------|------|
| Claude   | `~/.claude/projects/<encoded-dir>/<uuid>.jsonl` | same prefix |
| Codex    | `~/.codex/sessions/YYYY/MM/DD/<id>/` and `~/.codex/history.jsonl` | `~/.codex/log/codex-tui.log` |
| Kimi     | `~/.kimi/sessions/<dir-hash>/<id>/context.jsonl` | `~/.kimi/logs/kimi.log` plus `wire.jsonl` |

### Model catalog

Per-provider model lists are sourced through [`claudine/lib/src/model_catalog/provider_sources.rs`](../../lib/src/model_catalog/provider_sources.rs):

- `static_catalog_for_provider()` — Claude and Codex use static lists derived from the generated `unchained-ai` enums.
- `fetch_provider_catalog()` — OpenCode and Qwen shell out to `opencode models`. Other providers return an empty list and rely entirely on user overrides.

The merged catalog is cached at `~/.claudine/cache/models/<provider>.json` and falls back to the stale cache when refresh fails.

### CLI parameter mapping

The seam between user input and the wrapper is the argv pre-parser in [`claudine/cli/src/argv.rs`](../../cli/src/argv.rs) (`argv::normalize`). Per-provider concerns:

- **Provider booleans** (`--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`, `--qwen`, `--roo`) are rewritten to `--provider <slug>` on composition subcommands. Adding a new provider means extending the rewrite table here.
- **`COMPOSITION_FLAGS_WITH_VALUE`** must be kept in sync with the value-bearing clap surface of `ComposeArgs` and `SequenceArgs`. The drift-detection test `composition_flags_with_value_matches_clap_surface` enforces this.

### ACP support

Claudine does not currently consume Agent Client Protocol (ACP) anywhere. There is no `supports_acp()` method on `Provider` and no `AcpCapability` field on `AgentCapabilities`. The only places ACP appears in the codebase are documentary:

- The Kimi Code agent descriptor lists `"ACP server mode"` in `scripts.hook_or_notify_mechanisms`.
- The Goose `event_support_level` comment notes that `HumanInTheLoop` is captured via the `request_permission` ACP stream message — but the actual capture is wrapper-based, not ACP-based.

If/when ACP is added, the natural home is a new `AcpCapabilities` sub-struct on `RuntimeCapabilities` plus a new `EventSupportLevel::Acp` variant.

### Future Improvements to Metadata

The following gaps were observed while preparing this document. Each represents work that would either reduce drift, simplify onboarding a new provider, or surface today's implicit knowledge as code:

1. **No central provider registry.** Per-provider metadata is intentionally split across `agents/`, `events/`, `linking/capabilities.rs`, `stream/`, `mcp/`, `model_catalog/`, and `cli/.../wrap/profile.rs`. A nine-provider audit currently means visiting at least seven files. A thin "is everything wired?" trait or build-time test that asserts each `Provider` variant has an entry in every module would catch drift early.
2. **`AgentCapabilities` is descriptive, not executable.** Fields like `entrypoints: Vec<&'static str>` are human-readable strings (`"codex exec"`). The wrapper profile re-encodes the same information as runtime behavior. A typed bridge (e.g. `entrypoint_subcommand: Option<&'static str>`) would let the wrapper consume the catalog directly.
3. **No first-class ACP support.** See above. A typed `AcpCapabilities { server_mode_supported, client_supported, events_via_acp }` would let `event_support_level` distinguish ACP-based capture from generic `NonHook`.
4. **Logging is documented but not consumed.** `LoggingCapabilities.session_locations` is a `Vec<&'static str>` of glob-like patterns. There is no `log_path_for_session()` helper that resolves them. `claudine logs` operates exclusively on Claudine's own JSONL — surfacing native session logs would benefit from typed templates.
5. **Output-format support is asymmetric across surfaces.** `NonInteractiveCapabilities.output_formats` carries strings like `"jsonl (--json)"`, while `WrapperProfile::apply_output_format()` is a runtime function. A typed `enum NativeOutputFormat { Text, Json, StreamJson, Jsonl, … }` plus per-provider mapping would replace both.
6. **Native event names live in a giant `match`.** `Provider::native_event_name()` is one match per `Provider × AgenticEvent` pair. Three providers (Claude, Gemini, OpenCode) are already migrated to `SharedNativeEventMapping` tables; the remaining five would benefit from the same data-driven table to make event-name research self-evident in code review.
7. **Confidence/gaps fields are advisory.** `ConfidenceProfile.gaps` is a `Vec<&'static str>` of free-form sentences. A more structured form (`gaps: Vec<KnownGap>` referencing files or trackers) would make audits scriptable.
8. **`PromptArgConventions` is duplicated knowledge.** The struct in `profile.rs` overlaps with `NonInteractiveCapabilities.entrypoints` and could be promoted into the agent catalog.
9. **Sniff coverage is implicit.** `Provider::sniff_ai_cli()` returns a `sniff::programs::AiCli`. There is no compile-time guarantee that a new `Provider` variant has a corresponding `AiCli` variant — it's a runtime panic risk if someone forgets to update `sniff` first.
10. **No discoverable model catalog source flag.** `provider_sources.rs` uses a hard-coded match instead of a `ModelSource` field on `Provider` or `AgentCapabilities`. The fact that "OpenCode shells out to `opencode models`" is buried in a function rather than declared.

## Checklist

The following sequence is the minimum required to add a new provider end-to-end. Items marked **(metadata only)** are pure data; everything else involves writing or modifying behavior.

### 1. Identifier and detection

- [ ] Add a variant to `sniff::programs::AiCli` (in the `sniff` package) and ship it before opening the Claudine PR.
- [ ] Add a variant to `enum Provider` in [`events/provider.rs`](../../lib/src/events/provider.rs) with `#[non_exhaustive]` already present.
- [ ] Append the variant to `PROVIDERS_DISPLAY_ORDER` in display order (matrix tests will fail otherwise).
- [ ] Implement (extend the `match` in) every method on `Provider`:

    - `cli_aliases()`
    - `as_slug()`
    - `agent_offset()`
    - `sniff_ai_cli()`
    - `docs_url()`
    - `usage_dashboard_url()`
    - `supports_skills()`
    - `event_support_level()` and `native_event_name()`
    - `shared_native_mappings()` (populate a `*_SHARED_NATIVE_MAPPINGS` table — strongly preferred over inline matches)

- [ ] Update `detect_from_payload()` if the provider has a recognizably distinct payload shape.
- [ ] Add a `Display` arm and a snake-case serialization test.

### 2. Capability catalog **(metadata only)**

- [ ] Add an `AgentId` variant in [`agents/model.rs`](../../lib/src/agents/model.rs) and append to `AgentId::ALL`.
- [ ] Update `AgentId::as_str()` and `AgentId::aliases()`.
- [ ] Create `agents/<provider>.rs` modeled on the existing files. Populate `AgentMeta`, `AgentDocs`, `ConfigCapabilities`, `RuntimeCapabilities` (model, non-interactive, system prompt, permissions, reasoning, logging, billing), `SkillsCapabilities`, `SlashCommandCapabilities`, `SubagentCapabilities`, `ScriptCapabilities`, `ConfidenceProfile`. Use `Confidence::Low` and explicit `gaps` strings for anything unverified.
- [ ] Wire the new agent into [`agents/registry.rs`](../../lib/src/agents/registry.rs) (`OnceLock`, `agent_for`, `all_agents`).
- [ ] Re-export the agent struct from [`agents/mod.rs`](../../lib/src/agents/mod.rs).

### 3. Hook integration

- [ ] Create `adapters/<provider>.rs` to parse inbound hook payloads into `AgenticEvent`s.
- [ ] Create `config/<provider>.rs` implementing `AgentConfigurator`. If the provider has no config-file hook surface, override `supports_config_registration() -> false`.
- [ ] Wire both into `adapters/mod.rs` and `config/mod.rs`.
- [ ] Update `ClaudineConfig` defaults if the provider needs unusual sample wiring.

### 4. Wrapper profile

- [ ] Create a `<Provider>Wrapper` unit struct in [`cli/src/commands/wrap/profile.rs`](../../cli/src/commands/wrap/profile.rs).
- [ ] Implement the `WrapperProfile` trait, paying particular attention to `apply_yolo`, `apply_entrypoint`, `apply_system_prompt`, `prompt_delivery`, `prompt_arg_conventions`, `apply_structured_stream`, `stdout_noise_prefixes`, and `build_resume_args`.
- [ ] Register the wrapper in `profile_for_provider()`.
- [ ] Add the matching `--<provider>` boolean to the argv normalizer's Rule 1 in [`cli/src/argv.rs`](../../cli/src/argv.rs).
- [ ] Add the new `claudine <provider>` subcommand.
- [ ] Update `COMPOSITION_FLAGS_WITH_VALUE` if the provider introduces new value-bearing flags.

### 5. Stream parsing

- [ ] Add a `stream/protocol/<provider>.rs` module with a tagged `*Event` enum (every field `#[serde(default)]`, no `deny_unknown_fields`).
- [ ] Add a `stream/<provider>_semantic.rs` parser implementing `SemanticStreamParser` with a two-pass `feed_line` (`Value` first, then typed deserialize).
- [ ] Update `stream_protocol_for()` and `create_semantic_parser()` in [`stream/mod.rs`](../../lib/src/stream/mod.rs).
- [ ] Ship the `unknown_event_type_fails_typed` test alongside per-variant deserialization tests.

### 6. Linking and resource portability

- [ ] Add a `<provider>_capabilities()` arm in [`linking/capabilities.rs`](../../lib/src/linking/capabilities.rs) and `capabilities_for()` / `all_capabilities()`.
- [ ] Add the corresponding `*_SCHEMA: ResourcePropertySchema` constants and reference the cross-referencing doc path.
- [ ] Update the cross-referencing matrix tests so they cover the new provider.

### 7. MCP support (optional)

- [ ] If the provider exposes MCP, implement import/export logic in [`mcp/import.rs`](../../lib/src/mcp/import.rs) and [`mcp/export.rs`](../../lib/src/mcp/export.rs) and update `mcp/state.rs`.
- [ ] If runtime injection is feasible (shadow `HOME`, env-var-based config, etc.), wire it into [`mcp/inject.rs`](../../lib/src/mcp/inject.rs); otherwise rely on the export-and-apply fallback.

### 8. Model catalog

- [ ] If a static list applies, extend `static_catalog_for_provider()` in [`model_catalog/provider_sources.rs`](../../lib/src/model_catalog/provider_sources.rs).
- [ ] If a dynamic source applies, extend `fetch_provider_catalog()`.
- [ ] Otherwise leave the empty default — user overrides will populate the catalog.

### 9. Documentation

- [ ] Add `claudine/docs/research/hooks/<provider>.md`.
- [ ] Add `claudine/docs/research/cross-referencing/<provider>.md`.
- [ ] Update `claudine/docs/topics/composition.md`, `claudine/docs/mcp-support.md`, and the `.claude/skills/claudine/` skill catalog if behavior diverges.
- [ ] Refresh provider tables in the README and any `--describe` / `--mapping` output snapshots.

### 10. Verification

- [ ] `just test` (claudine area), with focus on `events::matrix`, `linking::capabilities::tests`, `stream::protocol::*::tests`, and `argv::tests`.
- [ ] `just lint` and `just doctest`.
- [ ] Smoke-test `claudine providers`, `claudine hooks --support`, `claudine hooks --mapping`, `claudine hooks --describe`, and `claudine init --quick` to confirm the new provider appears in matrix output.
- [ ] If the binary is installed, run `claudine <provider>` against a trivial prompt to validate the wrapper end-to-end.

## Things to Look Out For

These are the most common failure modes and surprises observed while integrating the eight existing providers.

### Provider identity is split across packages

A new `Provider` variant compiles only after `sniff::programs::AiCli` carries a matching variant — `Provider::sniff_ai_cli()` returns `AiCli` directly. Add the variant in `sniff` and publish/path-link the change before touching Claudine. Likewise, the `Provider` enum is `#[non_exhaustive]` and tests rely on `PROVIDERS_DISPLAY_ORDER` being kept in lockstep with display order; forgetting to extend the constant produces a silent under-count in matrix reports.

### Hook capture is tri-modal

There are three completely different mechanisms for getting events out of a provider:

- **Native hooks** (config-file based — Claude, Gemini, OpenCode plugins, Codex `notify`).
- **Stream parsing** (Goose, Qwen, Codex JSONL, Roo Code emitter).
- **Wire-mode proxy** (Kimi `--wire`).

`EventSupportLevel` distinguishes `Hook` from `NonHook` but does not name *which* non-hook mechanism is in play. When you see `NonHook` in the matrix you must read the configurator and the adapter to know whether claudine intercepts the CLI launch, parses its stdout, or proxies a JSON-RPC channel. Plan the architecture decision early — it shapes the rest of the wrapper profile.

### Native event names overlap across providers

Claude and Gemini both use `hook_event_name` as the payload key, but with disjoint native names. `Provider::detect_from_payload()` resolves the ambiguity by checking which provider's mapping table claims the name, and ambiguous cases default to Claude. When you add a new provider that uses a generic key (`event`, `type`, `event_type`, `event_name`, `method`), test detection against representative payloads from every existing provider to make sure your new arm is more specific than the existing fall-through.

### Argv pre-parsing is order-sensitive

The four rules in `argv::normalize` run in a fixed order: Rule 1 (provider booleans → `--provider <slug>`) → Rule 2 (canonicalize `--provider`) → Rule 4 (hoist `--help`) → Rule 3 (insert `--` before setter). Rule 4 must run before Rule 3 or a trailing `--help` will be absorbed by the setter region. The normalizer is a *strict no-op* under `COMPLETE` (shell completion), after the first literal `--`, on non-UTF-8 tokens, for argv with fewer than two elements, and on non-composition subcommands. New rules must respect those guards or shell completion will break in subtle ways.

### Wrapper and metadata can drift

The agent capability catalog (`agents/<provider>.rs`) is descriptive — `entrypoints: vec!["codex exec"]` is a string for humans. The wrapper profile (`cli/.../profile.rs`) re-encodes the same information as runtime behavior. There is no compile-time guarantee these two representations agree. When you change a flag (e.g. Claude moved from `--system-prompt` to `--system-prompt-file` for non-interactive mode), update *both* the runtime mapper *and* the descriptor.

### YOLO is a per-provider negotiation

Claudine canonicalizes `--yolo` / `-y` and forwards a different native flag per provider. Several providers also support YOLO natively under the same `--yolo` name (Kimi, Gemini, Codex via alias). The pattern that works:

1. `apply_yolo()` injects the canonical native flag.
2. `reject_direct_yolo()` blocks users from passing the native flag directly so behavior stays predictable.
3. `apply_yolo_for_mode()` overrides 1 when interactive vs. non-interactive matters (OpenCode forwards `--dangerously-skip-permissions` only in non-interactive mode).

OpenCode does not support YOLO at all in interactive mode; trying to "fix" that by forwarding the flag breaks the TUI.

### System-prompt delivery is provider-specific

Every wrapper has its own answer to "where does the system prompt live?":

- Claude: `--append-system-prompt` (interactive) vs `--append-system-prompt-file` (non-interactive); plus `--system-prompt` / `--system-prompt-file` for full replacement.
- Codex: `model_instructions_file` setting plus `AGENTS.override.md` precedence.
- Gemini (replace mode): write a temp file and set `GEMINI_SYSTEM_MD` env var.
- Gemini (append mode): build a shadow `HOME` containing a `.gemini/GEMINI.md` and override `HOME=`.
- Kimi: replace mode requires writing a temp prompt file *and* a temp agent YAML pointing at it via `--agent-file`. Append mode is unsupported.
- OpenCode: full replacement is unsupported; only `AGENTS.md`-style supplementation works.

The `PreparedSystemPrompt` / `SystemPromptApplication` types (in [`cli/.../wrap/system_prompt.rs`](../../cli/src/commands/wrap/system_prompt.rs)) accommodate args, env, temp files, temp dirs, and warnings. New wrappers should reuse those rather than printing directly.

### Stream protocol drift is silent

Provider stream formats evolve frequently. The protocol modules deliberately use `#[serde(default)]` on every field and avoid `deny_unknown_fields`, and the parser falls back to silent skip on unknown event types. The trade-off is that *missing* fields produce silent zeros instead of errors. Always add a per-variant test plus an `unknown_event_type_fails_typed` test for the new provider, and double-check the helper methods (`resolved_tool_name`, `take_input`, `effective_cost_usd`, etc.) when adding aliases — handlers see only the resolved value, so the alias plumbing is invisible at the call site.

Special cases already in production:

- Claude rate-limit status names drifted in 2026-04-18 (`approaching_limit` / `limited` → `allowed_warning` / `rejected`). The Claude formatter still passes both schemes through. New providers should plan for an analogous compatibility shim.
- OpenCode's `Reasoning` event has both a top-level `text` and a nested `part.text`. The typed enum carries both so the parser can prefer the most specific variant.
- The Gemini parser had to be patched for markdown-list rendering when reasoning blocks appear.

### Non-interactive output is a 9-section contract

`LiveSemanticSink` enforces strict ordering of stderr output: execution line, env, system prompt, agent prompt, session ID, thinking prose, tool/info events, final STDOUT, metadata. New wrappers must route their events through the existing semantic pipeline; printing directly to stderr breaks the contract and produces visually inconsistent output. Tool calls in particular are formatted as `→ Name(summary)` / `← Name(slot)` via `ToolCallDisplay`; raw JSON dumps must never reach the terminal.

### Hook handlers run under a deadline

`claudine handle` enforces a 5-second hook deadline (overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`). Bash and messenger actions inside handlers have their own 3-second timeout. If a new provider introduces synchronous network calls in its handler path, the deadline will fire and exit 124. Any blocking IO must be async, time-boxed, or moved to a background sink.

### Linking compatibility cuts both ways

Providers don't just *write* their resources to their own directories — most also *read* from neighbors. OpenCode reads `.claude/skills`, Goose reads `.claude/skills` and `.agents/skills`, Kimi reads `.claude/skills` *and* `.codex/skills`. When you add a provider, decide both:

1. What does *it* read from? (populate `also_reads_from` and `PathDiscovery::user_paths`/`project_paths`).
2. What format does it want? (populate `ResourceFormat`, `SkillFrontmatter`, and `ResourcePropertySchema`).

Linker conflict reports rely on this metadata being accurate; getting it wrong produces false-positive "incompatible" markings or, worse, silent silent skips.

### Roo Code is the canonical edge case

Roo Code is included in `Provider`, `AgentId`, and the linking matrix but has no `WrapperProfile` — `profile_for_provider(Provider::RooCode)` returns `None` because it runs as a VS Code extension. Treat it as a reminder: not every provider has a binary. If a new provider follows the same pattern (Cursor agent panel, IntelliJ plugin, Zed assistant, …), do the metadata work and skip the wrapper rather than writing a no-op profile.

### Test coverage is the safety net

The most important regression tests for any new provider:

- `events::matrix::tests::support_matrix_matches_provider_api` — catches missing `event_support_level` arms.
- `linking::capabilities::tests::all_providers_have_capabilities` — catches missing `capabilities_for` arms.
- `stream::protocol::<provider>::tests::unknown_event_type_fails_typed` — pins the format-evolution contract.
- `argv::tests::composition_flags_with_value_matches_clap_surface` — catches drift between the pre-parser and the clap surface.
- The matrix snapshot tests under `claudine/cli/tests/` — catch unintended changes to `--support`, `--mapping`, and `--describe` output.

If any of these fail in CI, the failure is almost always pointing at a real omission. Resist the urge to update the snapshot without first verifying the new behavior is correct.
```
