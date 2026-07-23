---
prompt: |-
    The claudine package wraps a number of popular agentic CLI providers including Claude Code, Codex, Qwen CLI, Kimi 
    CLI, Gemini CLI, Goose, and OpenCode. 
    
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

Claudine supports seven agentic CLI providers today, and the architecture is deliberately built so that an eighth provider should mostly be a matter of populating typed metadata and implementing a small number of focused traits. This document walks through where each piece of provider-specific information lives in the codebase, what gaps still exist, and the practical steps and pitfalls of adding a new provider.

## Metadata

Provider knowledge is intentionally fragmented across several modules so that each concern (hooks, streaming, linking, wrapping, MCP, model catalog, logging) can evolve independently. The flip side of that decision is that "metadata for provider X" is not a single struct — it is a coordinated set of definitions that all reference the same `Provider` enum variant.

### `Provider` enum — the canonical identifier

The `Provider` enum in [`claudine/lib/src/provider/identity.rs`](../../lib/src/provider/identity.rs) is the canonical identifier used everywhere else. It is the *minimum* surface a new provider must implement; nothing else compiles until this entry exists. The `impl Provider` blocks (CLI aliases, sniff binding, payload detection, slug, skills support, doc URLs, agent offset, event-mapping accessors, and `Display`) live alongside the enum in [`claudine/lib/src/provider/methods.rs`](../../lib/src/provider/methods.rs).

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

### Provider catalog (`ProviderInfo`)

The richer descriptive metadata lives under [`claudine/lib/src/provider/`](../../lib/src/provider/): one `<NAME>_INFO: ProviderInfo` constant per provider in `provider/<slug>/data.rs` (**generated** by `claudine-gen` — never hand-edited), with the four behavior-trait impls beside it in `provider/<slug>/behavior.rs`. The typed catalog fields on `ProviderInfo` (identity, path templates, output formats, entrypoints, system-prompt/YOLO/reasoning descriptors, known gaps, ACP support, prompt-arg conventions, model catalog data) are the descriptive surface; `claudine providers --describe --format json` serializes them.

The central `ProviderInfo` registry lives in [`provider/registry.rs`](../../lib/src/provider/registry.rs) (`provider_info(Provider)`, `all_providers()`). The legacy `agents::AgentCapabilities` string-heavy 80-field tree, its `Agent` trait, and the `agent_for` forwarding registry were retired in Phase C of the provider-metadata workstream (2026-07); the typed catalog fields replaced every live consumer.

### Hook events and native-name mappings

Event metadata lives in [`claudine/lib/src/events/`](../../lib/src/events/). The 16-variant `AgenticEvent` enum in [`events/agentic_event.rs`](../../lib/src/events/agentic_event.rs) is the unified lifecycle vocabulary. New providers do not extend this enum; they project their native events onto it.

Two pieces of per-provider event metadata are required, and both are now served through `ProviderInfo.event_mapping` ([`provider/event_mapping.rs`](../../lib/src/provider/event_mapping.rs)):

- **Support level** — `EventMappingTable::support_level(event)` returns `EventSupportLevel::{Hook, NonHook, Acp, NotSupported}` for each `AgenticEvent`. `Hook` means "registerable via config-file modification"; `NonHook` means "captured via wrapper, wire-mode proxy, or stream parsing"; `Acp` means "captured via the Agent Client Protocol surface"; `NotSupported` is unreachable. `Provider::event_support_level()` is a thin forwarder.
- **Native names** — `EventMappingTable::native_name(event)` returns the provider's own string identifier for each event, and `EventMappingTable::registration_native_name(event)` filters that to rows that participate in standard hook registration. The same table also carries parse aliases used by `event_from_native_name()`. Each provider's mapping table is owned by its `provider/<name>.rs` module so registration and adapter parsing cannot drift apart.

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
- Per-provider import/sync logic lives in [`claudine/lib/src/mcp/import.rs`](../../lib/src/mcp/import.rs) and [`mcp/state.rs`](../../lib/src/mcp/state.rs) — only the providers wired in actually appear (Claude, Codex, Gemini, OpenCode at the time of writing).
- Per-provider runtime injection logic lives in [`mcp/inject.rs`](../../lib/src/mcp/inject.rs); only Codex, Gemini, and OpenCode currently implement runtime injection. Other providers print a guidance message pointing at `claudine mcp export <provider> --apply`.

### Logging and reporting

Claudine's own logs are provider-agnostic and live under `~/.claudine/logs/` — daily-rotated JSONL files (`YYYY-MM-DD.jsonl`) and a SQLite metrics index at `metrics.db`. Path resolution is centralized in [`claudine/lib/src/reporting/paths.rs`](../../lib/src/reporting/paths.rs).

The provider's *own* logs are documented (not consumed) through the agent catalog — `session_log_paths` carries typed per-session transcript templates. Examples encoded today:

| Provider | Sessions | Logs |
|----------|----------|------|
| Claude   | `~/.claude/projects/<encoded-dir>/<uuid>.jsonl` | same prefix |
| Codex    | `~/.codex/sessions/YYYY/MM/DD/<id>/` and `~/.codex/history.jsonl` | `~/.codex/log/codex-tui.log` |
| Kimi     | `~/.kimi/sessions/<dir-hash>/<id>/context.jsonl` | `~/.kimi/logs/kimi.log` plus `wire.jsonl` |

### Model catalog

Per-provider model lists are sourced through [`claudine/lib/src/model_catalog/provider_sources.rs`](../../lib/src/model_catalog/provider_sources.rs):

- `expected_baseline()` — the validation baseline: generated expected-offering ids plus their rolling aliases.
- `fetch_provider_catalog()` — OpenCode shells out to `opencode models`; providers without a listing source return an empty list. The fetched listing feeds only the drift-channel cache at `~/.claudine/cache/models/<provider>.json`, never validation.

### CLI parameter mapping

The seam between user input and the wrapper is the argv pre-parser in [`claudine/cli/src/argv.rs`](../../cli/src/argv.rs) (`argv::normalize`). Per-provider concerns:

- **Provider booleans** are derived from the compiled provider catalog and rewritten to `--provider <slug>` on composition subcommands. Adding a catalog provider makes its shorthand available without a hand-maintained rewrite table.
- **`COMPOSITION_FLAGS_WITH_VALUE`** must be kept in sync with the value-bearing clap surface of `ComposeArgs` and `SequenceArgs`. The drift-detection test `composition_flags_with_value_matches_clap_surface` enforces this.

### ACP support

Phase 7 of the centralized providers refactor introduced first-class ACP metadata. `ProviderInfo.acp` is an [`AcpSupport`](../../lib/src/provider/acp.rs) descriptor (`server_mode`, `client_supported`, `events`) and `EventSupportLevel::Acp` is a real variant in [`provider/event_mapping.rs`](../../lib/src/provider/event_mapping.rs). Goose's `request_permission` and Kimi's `ApprovalRequest` are mapped as `Acp` rows; an invariant test asserts that any `Acp` row implies a non-`NotSupported` `AcpSupport::server_mode`.

`claudine hooks --capture-method` surfaces this metadata at the CLI surface; runtime ACP consumption (proxy, server) is still pending.

### Migration History

The centralized-providers refactor (`features/2026-04-26-centralized-providers/`) closed nearly all of the gaps that this document originally listed as "future improvements":

- **Phase 0** unified `AgentId` and `Provider` into a single `Provider` enum.
- **Phase 1** introduced `crate::provider`, the `ProviderInfo` struct, and the four behavior traits (`ProviderBehavior`, `McpBehavior`, `AdapterBehavior`, `ConfiguratorBehavior`). `provider_info(p)` is the single registry that all per-domain dispatch flows through.
- **Phase 2** moved `AgentCapabilities` and `ProviderCapabilities` data into per-provider `provider/<name>.rs` modules; `agents/<name>.rs` thin facades and the per-variant agent constructors were retired.
- **Phase 3** consolidated event support level, native names, parse aliases, and registration metadata into the per-provider `EventMappingTable`. The `SharedNativeEventMapping` constants and the giant `Provider::event_support_level` / `Provider::native_event_name` matches were replaced by table lookups.
- **Phase 4** routed stream parser construction, MCP operations, inbound payload parsing, and hook configurator dispatch through the four behavior traits.
- **Phase 5** replaced descriptive `Vec<&'static str>` capability fields with typed catalog data: `PathTemplate`, `OutputFormatSupport`, `EntrypointSpec`, `SystemPromptSpec`, `YoloSupport`, `ReasoningSupport`, and `KnownGap`.
- **Phase 6** thinned `WrapperProfile` so ordinary behavior reads from `provider_info` and the composition flag drift surface is derived from clap at runtime.
- **Phase 7** added the `AcpSupport` descriptor, the `EventSupportLevel::Acp` variant, and the `claudine hooks --capture-method` output.
- **Phase 8** consolidated the `impl Provider` blocks (CLI aliases, sniff binding, payload detection, slug, doc URLs, agent offset, event mapping accessors, `Display`) into [`provider/methods.rs`](../../lib/src/provider/methods.rs) and retired the per-provider thin facade structs that previously lived in `agents/<name>.rs`. The `crate::events::Provider` and `events::PROVIDERS_DISPLAY_ORDER` re-exports remain in place as `#[deprecated]` shims (see [`claudine/lib/src/events/provider.rs`](../../lib/src/events/provider.rs)); the `AgentId` alias was removed with the `agents` module at the AgentCapabilities retirement (provider-metadata Phase C, 2026-07). All in-repo consumers now import from `crate::provider::*`.

Open items that remain advisory rather than typed:

- Native session/log locations are typed path templates (`session_log_paths`), but resolution helpers for native session paths are still TBD.

## Checklist

After the centralized-providers refactor, adding a ninth provider has a much smaller surface area. The minimum required edits are:

### 1. Identifier and detection

- [ ] Add a variant to `sniff::programs::AiCli` (in the `sniff` package) and ship it before opening the Claudine PR.
- [ ] Add a variant to `enum Provider` in [`provider/identity.rs`](../../lib/src/provider/identity.rs) and append it to `PROVIDERS_DISPLAY_ORDER`.
- [ ] Extend the `match` arms in [`provider/methods.rs`](../../lib/src/provider/methods.rs): `cli_aliases()`, `as_slug()`, `agent_offset()`, `sniff_ai_cli()`, `docs_url()`, `usage_dashboard_url()`, `supports_skills()`, `Display`, and `detect_from_payload()` (if the provider has a recognizably distinct payload shape).

### 2. Central provider definition

- [ ] Create [`provider/<name>.rs`](../../lib/src/provider/) modeled on existing entries. Populate the `<NAME>_INFO: ProviderInfo` constant with:

    - Identity (`display_name`, `slug`, `binary`, `agent_offset`, `cli_aliases`, `docs_url`, `usage_dashboard_url`, `sniff_binding`, `supports_skills`).
    - `event_mapping: &EventMappingTable` describing every supported `AgenticEvent` row (support level, native name, parse aliases, registration target).
    - The four behavior fields (`behavior`, `mcp`, `adapter`, `configurator`). Implement only what the provider supports; defaults return typed `NotSupported`.
    - The `resource_support_fn` accessor backed by a per-provider `LazyLock<ProviderCapabilities>`.
    - Phase 5 typed catalog data: `session_log_paths`, `config_paths`, `memory_files`, `output_formats`, `entrypoints`, `system_prompt`, `yolo`, `reasoning`, `known_gaps`, `acp`, `prompt_arg_conventions`.

- [ ] Register `&<NAME>_INFO` in [`provider/registry.rs`](../../lib/src/provider/registry.rs).
- [ ] The exhaustiveness tests in [`provider/tests.rs`](../../lib/src/provider/tests.rs) auto-detect the new variant; rerun them.

### 3. Stream parsing (if applicable)

- [ ] Add [`stream/protocol/<provider>.rs`](../../lib/src/stream/protocol/) with a tagged `*Event` enum (every field `#[serde(default)]`, no `deny_unknown_fields`).
- [ ] Add `stream/<provider>_semantic.rs` implementing `SemanticStreamParser` with a two-pass `feed_line` (`Value` first, then typed deserialize).
- [ ] Implement `ProviderBehavior::create_semantic_parser` on the new provider's behavior struct in `provider/<name>.rs`.
- [ ] Ship the `unknown_event_type_fails_typed` test alongside per-variant deserialization tests.

### 4. Wrapper profile (if a CLI binary exists)

- [ ] Create a `<Provider>Wrapper` unit struct in [`cli/src/commands/wrap/profile.rs`](../../cli/src/commands/wrap/profile.rs). Lean on the trait's catalog-derived defaults wherever the provider's behavior matches the catalog; only override the irreducible quirks.
- [ ] Register the wrapper in `wrapper_for(Provider)`.
- [ ] Add the matching `--<provider>` boolean to the argv normalizer's Rule 1 in [`cli/src/argv.rs`](../../cli/src/argv.rs).
- [ ] Add the new `claudine <provider>` subcommand.

### 5. MCP and model catalog (optional)

- [ ] If the provider exposes MCP, implement `McpBehavior` on the provider's behavior struct.
- [ ] If a static or dynamic model catalog applies, extend [`model_catalog/provider_sources.rs`](../../lib/src/model_catalog/provider_sources.rs).

### 6. Documentation

- [ ] Add `claudine/docs/research/hooks/<provider>.md` and `claudine/docs/research/cross-referencing/<provider>.md`.
- [ ] Update `.claude/skills/claudine/` if architecture or workflow guidance changes.
- [ ] Refresh provider tables in the README and any `--describe` / `--mapping` output snapshots.

### 7. Verification

- [ ] `just test` (claudine area), with focus on `events::matrix`, `provider::tests`, `linking::capabilities::tests`, `stream::protocol::*::tests`, and `argv::tests`.
- [ ] `just lint` and `just doctest`.
- [ ] Smoke-test `claudine providers`, `claudine hooks --support`, `claudine hooks --mapping`, `claudine hooks --describe`, `claudine hooks --capture-method`, and `claudine init --quick` to confirm the new provider appears in matrix output.
- [ ] If the binary is installed, run `claudine <provider>` against a trivial prompt to validate the wrapper end-to-end.

## Things to Look Out For

These are the most common failure modes and surprises observed while integrating the seven existing providers.

### Provider identity is split across packages

A new `Provider` variant compiles only after `sniff::programs::AiCli` carries a matching variant — `Provider::sniff_ai_cli()` returns `AiCli` directly. Add the variant in `sniff` and publish/path-link the change before touching Claudine. Likewise, the `Provider` enum is `#[non_exhaustive]` and tests rely on `PROVIDERS_DISPLAY_ORDER` being kept in lockstep with display order; forgetting to extend the constant produces a silent under-count in matrix reports.

### Hook capture is tri-modal

There are three completely different mechanisms for getting events out of a provider:

- **Native hooks** (config-file based — Claude, Gemini, OpenCode plugins, Codex `notify`).
- **Stream parsing** (Goose, Qwen, Codex JSONL).
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

### Not every provider has a binary

Roo Code, removed from the roster in 2026-07, was the canonical example: it lived in `Provider` and the linking matrix but had no `WrapperProfile` because it ran as a VS Code extension. If a new provider follows the same pattern (Cursor agent panel, IntelliJ plugin, Zed assistant, …), do the metadata work in `provider/<name>.rs` and skip the wrapper rather than writing a no-op profile.

### Test coverage is the safety net

The most important regression tests for any new provider:

- `provider::tests` — registry self-consistency, sniff binding round-trip, and `ProviderInfo` field exhaustiveness.
- `events::matrix::tests::support_matrix_matches_provider_api` — catches missing event mapping rows.
- `linking::capabilities::tests::all_providers_have_capabilities` — catches missing `resource_support` arms.
- `stream::protocol::<provider>::tests::unknown_event_type_fails_typed` — pins the format-evolution contract.
- `argv::tests::composition_flags_with_value_matches_clap_surface` — catches drift between the pre-parser and the clap surface.
- The matrix snapshot tests under `claudine/cli/tests/` — catch unintended changes to `--support`, `--mapping`, `--describe`, and `--capture-method` output.

If any of these fail in CI, the failure is almost always pointing at a real omission. Resist the urge to update the snapshot without first verifying the new behavior is correct.
```
