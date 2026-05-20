---
categies: 
    - technical
    - type-safety
    - goal-alignment
---
# Provider Metadata

## Provider Metadata

### Why Metadata is Important

Claudine normalizes eight agentic CLIs (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code) into a single configuration model. Each provider differs in dozens of material ways: binary name, config file paths, stream protocol, hook event names, YOLO flag, reasoning controls, system prompt delivery mechanism, model catalog source, and more. Without a type-strong, centralized catalog, these differences leak into scattered `match Provider { ... }` blocks across the codebase, creating a maintenance burden that grows with every new provider.

The `ProviderInfo` struct solves this by serving as the **single authoritative record** for every static fact about a provider. Each of the eight `Provider` variants maps to exactly one `&'static ProviderInfo` constant served from the central registry. The struct is intentionally non-optional for identity fields (`display_name`, `slug`, `binary`, `agent_offset`, `cli_aliases`, `docs_url`) so a compile-time gap is impossible. Behavior that is genuinely dynamic (payload detection, MCP import/export, stream parser construction, hook registration) lives behind four focused trait objects on the same struct, so a single registry lookup returns both data and behavior.

The design goal is that **all code variation between providers should be driven exclusively by `ProviderInfo` metadata**. When a feature needs to branch on provider identity, it should consult `provider_info(provider).<field>` or call a method on one of the four behavior traits, never match directly on `Provider` variants outside the registry.

### What Metadata We Capture

The `ProviderInfo` struct (`lib/src/provider/mod.rs`) carries 36 fields organized into three groups: identity, behavior traits, and typed catalog data.

#### Identity Fields

| Field | Type | Description |
|-------|------|-------------|
| `provider` | `Provider` | Canonical enum identifier (`Provider::Claude`, etc.) |
| `display_name` | `&'static str` | Friendly name for display ("Claude", "Kimi Code") |
| `slug` | `&'static str` | Snake-case identifier for file paths and JSON keys ("claude") |
| `short_name` | `&'static str` | Abbreviated name for logs and CLI output ("claude", "kimi") |
| `binary` | `&'static str` | Executable name on `$PATH` ("claude", "codex") |
| `agent_offset` | `&'static str` | Dot-prefixed config directory (".claude", ".codex") |
| `cli_aliases` | `&'static [&'static str]` | Accepted CLI alias forms (`["claude"]`, `["kimi", "kimicode", ...]`) |
| `docs_url` | `&'static str` | Provider documentation homepage |
| `usage_dashboard_url` | `Option<&'static str>` | Billing/usage dashboard URL, when available |
| `sniff_binding` | `AiCli` | Typed binding to `sniff::programs::AiCli` for install detection |
| `supports_skills` | `bool` | Whether the provider supports skill discovery |

#### Behavior Trait Objects (dynamic dispatch)

| Field | Trait | Purpose |
|-------|-------|---------|
| `behavior` | `ProviderBehavior` | Cross-cutting: payload detection, semantic parser construction |
| `mcp` | `McpBehavior` | MCP import/state/inject/export lifecycle |
| `adapter` | `AdapterBehavior` | Inbound payload parsing and provider adapter |
| `configurator` | `ConfiguratorBehavior` | Hook registration and config-file mutation |

#### Typed Catalog Data (strongly typed static facts)

| Field | Type | Description |
|-------|------|-------------|
| `stream_protocol` | `Option<StreamProtocol>` | Structured stream format (`StreamJson`, `Ndjson`, `Jsonl`, `WireJsonRpc`, `None`) |
| `event_mapping` | `&'static EventMappingTable` | Per-event support level, native names, parse aliases, registration targets |
| `session_log_paths` | `&'static [PathTemplate]` | Templates for per-session JSONL transcript files |
| `session_locations` | `&'static [PathTemplate]` | Templates for ancillary session-state directories |
| `config_paths` | `&'static [PathTemplate]` | User/project/local config file templates (first = primary user config) |
| `memory_files` | `&'static [PathTemplate]` | Memory/instruction files contributing to system prompt hierarchy |
| `output_formats` | `&'static [OutputFormatSupport]` | Supported non-interactive output formats with native names, CLI flags, selectors |
| `entrypoints` | `&'static [EntrypointSpec]` | Non-interactive (and selected interactive) entrypoints with subcommands and required flags |
| `system_prompt` | `&'static SystemPromptSpec` | Append/replace delivery mechanism, split by interactive/non-interactive mode |
| `yolo` | `YoloSupport` | YOLO/auto-approve mechanism (direct flag, env var, non-interactive-only, or none) |
| `reasoning` | `ReasoningSupport` | Reasoning controls (named levels, numeric budget, binary toggle, or provider-specific) |
| `known_gaps` | `&'static [KnownGap]` | Structured gaps in provider capability data, tagged by area |
| `acp` | `AcpSupport` | ACP server mode, client capability, and events captured via ACP |
| `prompt_arg_conventions` | `PromptArgConventions` | How the provider's CLI represents a prompt on argv |
| `static_models` | `&'static [&'static str]` | Compiled-in model catalog entries |
| `dynamic_source` | `ModelCatalogSource` | How dynamic model lists are sourced (`Static`, `OpencodeCli`, etc.) |
| `model_env_vars` | `&'static [&'static str]` | Provider-specific MODEL env var chain (e.g. `["CLAUDE_MODEL", "ANTHROPIC_MODEL"]`) |
| `cli_sensitive_axes` | `CliSensitiveAxes` | Which permission-policy axes CLI flags can override at runtime (10 boolean axes) |
| `repo_home_root_files` | `&'static [&'static str]` | Root-level files preserved during shadow-HOME isolation |

#### Supporting Type Modules

Each typed catalog field is backed by a dedicated module under `lib/src/provider/` with its own strongly typed enum or struct:

| Module | Key Types |
|--------|-----------|
| `event_mapping.rs` | `EventSupportLevel` (6 variants: `NotSupported`, `Hook`, `StreamParse`, `WireProxy`, `Acp`, `Wrapper`), `EventMapping`, `EventMappingTable`, `WireProxyMode` |
| `system_prompt.rs` | `SystemPromptDelivery` (6 variants), `SystemPromptCustomTag` (4 variants), `SystemPromptDeliveryByMode`, `SystemPromptSpec` |
| `yolo.rs` | `YoloSupport` (5 variants: `None`, `DirectFlag`, `DirectFlagWithAlias`, `NonInteractiveOnly`, `EnvVar`) |
| `reasoning.rs` | `ReasoningSupport` (6 variants), `ReasoningCustomTag` (4 variants) |
| `output_format.rs` | `OutputFormat`, `OutputFormatSupport`, `EntrypointMode`, `EntrypointSpec` |
| `path_template.rs` | `PathTemplate` (Static/Templated), `PathSegment` (11 variants), `PathContext`, `GlobKind` |
| `acp.rs` | `AcpServerMode` (3 variants), `AcpEvent` (6 variants), `AcpSupport` |
| `known_gap.rs` | `KnownGapArea` (13 variants), `KnownGap` |
| `prompt_args.rs` | `PromptArgConventions`, `COMMON_VALUE_TAKING_FLAGS` |
| `cli_sensitivity.rs` | `CliSensitiveAxes` (10 boolean fields + `NONE`/`ALL` constants) |
| `model_catalog_source.rs` | `ModelCatalogSource` (4 variants) |

### Technical Overview

#### Central Registry

`provider_info()` in `lib/src/provider/registry.rs` is the **only** authorized `match Provider` dispatch site in the lib crate. It maps each `Provider` variant (by `repr(usize)` index) to a `&'static ProviderInfo` constant defined in the per-provider module. The `OnceLock`-backed registry is initialized on first access and every entry lives in the binary's read-only data segment.

#### Per-Provider Modules

Each provider has a dedicated module (`claude.rs`, `codex.rs`, etc.) that defines:

1. A zero-sized behavior struct (e.g. `ClaudeProvider`) implementing all four behavior traits
2. Static constants for every typed catalog field
3. A single `pub(super) static FOO_INFO: ProviderInfo` constant wiring all fields together
4. `LazyLock`-backed accessors for `AgentCapabilities` and `ProviderCapabilities` (legacy facades)

#### Behavior Traits

The four behavior traits in `lib/src/provider/behavior.rs` provide default "not supported" implementations so providers only override what they need:

- **`ProviderBehavior`**: `detect_from_payload()` (required), `create_semantic_parser()` (defaults to Claude parser)
- **`McpBehavior`**: `supported()`, `runtime_injector()`, `discover_configs()`, `parse_config()`, `native_config_path()`, `read_existing_native_servers()`, `write_native_config()` — all default to no-op/empty
- **`AdapterBehavior`**: `detect()`, `parse_payload()`, `provider_adapter()` — defaults to false/None/panic
- **`ConfiguratorBehavior`**: `hooks_supported()`, `agent_configurator()` — defaults to false/panic

#### Legacy Compatibility Facades

Two fn-pointer fields bridge to the pre-refactoring capability systems:

- `agent_capabilities_fn`: Returns the legacy `AgentCapabilities` tree (used by `agents::registry::agent_for`)
- `resource_support_fn`: Returns the cross-provider linking `ProviderCapabilities` descriptor

Both are `#[serde(skip)]` so the JSON describe surface exposes only the typed catalog fields.

#### Re-exports

`provider/mod.rs` re-exports every public type from the supporting modules so downstream code can import from `claudine::provider::*`:

```rust
pub use acp::{AcpEvent, AcpServerMode, AcpSupport};
pub use behavior::{AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
pub use cli_sensitivity::CliSensitiveAxes;
pub use event_mapping::{EventMapping, EventMappingTable, EventSupportLevel};
pub use known_gap::{KnownGap, KnownGapArea};
pub use model_catalog_source::ModelCatalogSource;
pub use output_format::{EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport};
pub use path_template::{GlobKind, PathContext, PathSegment, PathTemplate};
pub use prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
pub use reasoning::{ReasoningCustomTag, ReasoningSupport};
pub use registry::{all_providers, provider_info};
pub use system_prompt::{SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec};
pub use yolo::YoloSupport;
// Plus Provider, OutputFormatSelector, PROVIDERS_DISPLAY_ORDER from provider_id
```

### What Additional Metadata We Could (or Should) Capture in the Future

#### 1. Config File Format

**Current state**: The legacy `AgentCapabilities` captures `ConfigFormat` (`Json`, `Jsonc`, `Toml`, `Yaml`, `Mixed`), but the typed `ProviderInfo` has no corresponding field. Config paths are listed as `PathTemplate` values, but the format each file uses is not described.

**Value**: Knowing the config format would let the MCP and hook registration layers serialize without hardcoding per-provider format logic inside their behavior implementations.

**Current pain**: MCP `write_native_config` implementations each contain format-specific serialization logic that cannot be generalized without this metadata.

**Effort**: Low. Add a `config_format: ConfigFormat` field to `ProviderInfo` and populate it from the legacy data. One field addition per provider module.

#### 2. Billing Model

**Current state**: The legacy `BillingCapabilities` struct captures billing models (`Subscription`, `PerToken`, `PrepaidCredits`, `ProviderOnly`) and notes, but the typed catalog does not expose this.

**Value**: `usage_dashboard_url` hints at billing, but the actual billing model is invisible to the typed surface. This affects features like cost estimation and session cost reporting.

**Current pain**: Billing model data exists only in the legacy tree; any new feature that needs it must go through the compatibility facade rather than the typed catalog.

**Effort**: Low. Add a `billing_models: &'static [BillingModel]` field (or a structured `BillingSupport` type) to `ProviderInfo`.

#### 3. Model Selection CLI Flags

**Current state**: The legacy `ModelCapabilities` captures `cli_flags`, `session_switch_commands`, `aliases`, `precedence_order`, and `notes` for model selection. The typed catalog captures `model_env_vars` and `static_models` but not the CLI flags used to inject a model at launch time.

**Value**: The `WrapperProfile::apply_model` method defaults to `--model <value>`, but several providers use different flags (Codex uses `--model`, Gemini uses `--model`, Kimi uses `--model` inside the wire envelope). Knowing the canonical flag per provider would let the default implementation handle all cases.

**Current pain**: Each wrapper profile overrides `apply_model` to inject provider-specific model flags, duplicating knowledge that belongs in the catalog.

**Effort**: Medium. Add a `model_cli_flag: Option<&'static str>` field. Migrate `WrapperProfile::apply_model` defaults. Requires coordination with the CLI crate's wrapper profile system.

#### 4. Sandbox/Container Support Descriptor

**Current state**: `WrapperProfile::apply_sandbox` is a per-provider override with no metadata backing. Each provider that supports sandboxing (Codex with `--sandbox-image`, Gemini with Docker) hardcodes its mechanism in the CLI wrapper.

**Value**: A typed `SandboxSupport` enum would let the wrapper's default `apply_sandbox` derive from the catalog instead of per-provider overrides.

**Current pain**: Adding sandbox support for a new provider requires a new wrapper profile override rather than a catalog entry.

**Effort**: Medium. Define a `SandboxSupport` enum (analogous to `YoloSupport`), add to `ProviderInfo`, migrate `WrapperProfile::apply_sandbox` defaults.

#### 5. Stdout/Stderr Noise Prefixes

**Current state**: `WrapperProfile::stdout_noise_prefixes` and `stderr_noise_prefixes` are trait methods returning `&'static [&'static str]`, defined per-provider in the CLI crate. They are not captured in the library's typed catalog.

**Value**: These are static facts about each provider's CLI output behavior. If captured in `ProviderInfo`, the library could reason about them (e.g. for reporting or composition pipelines) without depending on the CLI crate.

**Current pain**: The CLI wrapper profile system owns this knowledge exclusively. Library-level features that need to know about noisy output cannot access it.

**Effort**: Low. Add `stdout_noise_prefixes: &'static [&'static str]` and `stderr_noise_prefixes: &'static [&'static str]` to `ProviderInfo`. Move the constants from CLI wrapper modules to the per-provider lib modules.

#### 6. Resume Session Mechanism

**Current state**: `WrapperProfile::build_resume_args` and `supports_resume` are per-provider overrides. The typed catalog has `EntrypointSpec` entries that capture resume-related flags (`-c`, `-r`, `--resume`, `--resume-session`), but the full resume invocation pattern is not described.

**Value**: A structured `ResumeSupport` descriptor could generalize `build_resume_args` so adding resume for a new provider is a catalog entry rather than a new wrapper override.

**Current pain**: Each provider with resume support hardcodes its resume argv construction in the wrapper profile.

**Effort**: Medium. Define a `ResumeSpec` type capturing the resume flag and session-ID injection pattern. Add to `ProviderInfo` and derive `build_resume_args` from it.

#### 7. Interactive Inline Closure Support

**Current state**: `WrapperProfile::supports_interactive_inline_closure` returns a hardcoded bool per provider. This is a static fact with no catalog backing.

**Value**: Capturing this in the catalog would let the composition layer reason about inline closure feasibility without consulting the wrapper profile.

**Current pain**: Library-level composition code must ask the CLI wrapper whether inline closure is possible, coupling library logic to CLI infrastructure.

**Effort**: Trivial. Add `supports_interactive_inline_closure: bool` to `ProviderInfo`.

#### 8. Prompt Delivery Mechanism

**Current state**: `WrapperProfile::prompt_delivery` is the most complex per-provider override, returning `PromptDelivery` (`Stdin`, `AppendArgs`, `InsertArgs`, `WireRpc`). The typed catalog captures prompt flags via `PromptArgConventions` and entrypoints via `EntrypointSpec`, but the full delivery strategy is not described.

**Value**: A structured `PromptDeliverySpec` could encode the delivery pattern (positional, flag-based, stdin, wire-RPC) so the wrapper profile's default implementation handles most providers.

**Current pain**: Every wrapper profile overrides `prompt_delivery` with provider-specific logic. Adding a new provider requires understanding the delivery nuances and implementing the method from scratch.

**Effort**: High. Prompt delivery interacts with entrypoints, mode switching, and the wire-RPC protocol in complex ways. A full abstraction would need to handle positional injection after subcommands, flag-value extraction, stdin piping, and wire-RPC envelopes. This is the most impactful but also the most complex gap to close.

## Decentralized Provider Info

Despite the centralized catalog, several sites across the codebase still encode provider-specific facts imperatively rather than declaratively. The following inventory is maintained so authors know where drift lives today and can evaluate whether a new catalog field would let a site become provider-agnostic.

### Lib Crate

| File | Lines | Provider(s) singled out | What is hardcoded |
|------|-------|------------------------|-------------------|
| `lib/src/stream/providers/mod.rs` | 37–48 | All | Semantic-parser factory dispatch. Each `Provider` variant maps to a distinct parser type. This is an intentional factory site, but it means adding a new provider requires a new parser implementation plus a new factory arm. |
| `lib/src/composition/select.rs` | 171 | OpenCode | Hard error when no model is selected in non-TTY mode: `"OpenCode requires a model in non-interactive mode"`. This could move to a `model_required_in_non_tty: bool` catalog field. |
| `lib/src/linking/agents.rs` | 166, 426 | Claude | `if provider == Provider::Claude { continue; }` skips that exclude Claude from agent-linking loops (Claude is the canonical source). |
| `lib/src/linking/commands.rs` | 175, 505 | Claude | Same Claude-skip pattern for command linking. |
| `lib/src/linking/skills/native.rs` | 51 | Claude | Claude-skip in native skill linking. |
| `lib/src/linking/skills/portable.rs` | 194 | Claude | Claude-skip in portable skill missing-link checking. |

### CLI Crate — Wrapper Execution (`wrap/`)

| File | Lines | Provider(s) singled out | What is hardcoded |
|------|-------|------------------------|-------------------|
| `cli/src/main.rs` | 23–31 | All 8 variants | CLI command-to-provider mapping. Each `Commands::*` variant maps to its `Provider` enum variant. This is structural dispatch and cannot be avoided without changing the clap derive hierarchy. |
| `cli/src/commands/wrap/mod.rs` | 558–579 | OpenCode | OpenCode model resolution via `apply_opencode_model_resolution`. Replaces the generic trait path because OpenCode uses a wire-envelope model flag rather than a CLI `--model` flag. |
| `cli/src/commands/wrap/mod.rs` | 583–596 | OpenCode | Universal `--model` flag is skipped for OpenCode in non-interactive mode; interactive OpenCode with `--model` is special-cased. |
| `cli/src/commands/wrap/mod.rs` | 599–601 | OpenCode | Non-interactive validation is bypassed for OpenCode because it has its own model resolution. |
| `cli/src/commands/wrap/mod.rs` | 692 | Codex, Gemini | MCP shadow-home is only created for Codex and Gemini: `matches!(provider, Provider::Codex \| Provider::Gemini)`. |
| `cli/src/commands/wrap/mod.rs` | 1028 | Codex | `StructuredCodexOutput::prepare` is invoked only for Codex when structured output is requested. |
| `cli/src/commands/wrap/mod.rs` | 1289–1308 | Codex | Final assistant message is read from `--output-last-message` file and rendered to stdout after the stream ends. |
| `cli/src/commands/wrap/composition/mod.rs` | 865 | Codex, Gemini | Same MCP shadow-home gate as `wrap/mod.rs`. |
| `cli/src/commands/wrap/composition/mod.rs` | 1074–1090 | OpenCode | Same OpenCode model resolution as `wrap/mod.rs`. |
| `cli/src/commands/wrap/composition/mod.rs` | 1093–1119 | OpenCode | Same `--model` skip logic as `wrap/mod.rs`. |
| `cli/src/commands/wrap/composition/mod.rs` | 1122–1124 | OpenCode | Same validation bypass as `wrap/mod.rs`. |
| `cli/src/commands/wrap/composition/mod.rs` | 1254–1260 | Codex | `StructuredCodexOutput::prepare` for Codex (slightly different condition: also triggered for interactive inline). |
| `cli/src/commands/wrap/policy.rs` | 59 | Codex | `can_write` sandbox logic only for Codex. Checks `--dangerously-bypass-approvals-and-sandbox` or `--yolo`, then evaluates sandbox mode. |
| `cli/src/commands/wrap/policy.rs` | 179–229 | OpenCode, Codex | `build_structured_plumbing` branches on provider to build the correct semantic parser + stderr bridge: OpenCode gets `OpenCodeLogBridge`, Codex gets `CodexLogBridge`, all others get plain `ObservedSemanticSink`. |
| `cli/src/commands/wrap/harness_orch.rs` | 531–549 | Codex | Codex final assistant message rendering inside the harness loop (duplicate of `wrap/mod.rs`). |
| `cli/src/commands/wrap/repo_home.rs` | 147 | Codex | `needs_shadow_home` matches on `Provider::Codex` when codex repo prompts exist. |
| `cli/src/commands/wrap/repo_home.rs` | 190 | Codex | Repo-scoped resource materialization is Codex-only (`materialize_codex_prompts`). |
| `cli/src/commands/wrap/live_semantic_sink/mod.rs` | 391–397 | Codex | Codex tool-result display: suppress the summary line for successful non-file tools. |
| `cli/src/commands/wrap/live_semantic_sink/mod.rs` | 400–405 | Codex | Codex tool-result body rendering: show the raw body in `Normal` verbosity. |
| `cli/src/commands/wrap/live_semantic_sink/mod.rs` | 465 | OpenCode | OpenCode `step_phase` info events are suppressed from human-visible output. |
| `cli/src/commands/wrap/inline.rs` | 9–16 | Codex, Gemini, OpenCode | `strip_prompt_tags_for_provider` strips MCP tags only for Codex, Gemini, and OpenCode. |
| `cli/src/commands/wrap/composition/structured.rs` | 119 | Codex | `had_streamed_assistant` explicitly excludes Codex (`provider != Provider::Codex`). |
| `cli/src/commands/wrap/composition/legacy_goose.rs` | 69–77 | Codex | Codex last-message file extraction in the legacy (non-structured) branch. |

### CLI Crate — WrapperProfile Trait Overrides

The `WrapperProfile` trait intentionally allows per-provider overrides for behavior that does not yet fit in the typed catalog. The following methods are overridden by one or more providers:

| Method | Overridden By | Could Be Catalog-Driven? |
|--------|--------------|--------------------------|
| `prompt_delivery` | All 8 providers | **High effort**. Delivery interacts with entrypoints, mode switching, and wire-RPC. |
| `build_resume_args` | Claude, Codex, Kimi, Qwen | **Medium effort**. A `ResumeSupport` descriptor with flag + session-ID pattern would cover most cases. |
| `apply_sandbox` | Codex, Qwen | **Low effort**. A `SandboxSupport` enum analogous to `YoloSupport` would suffice. |
| `supports_interactive_inline_closure` | Codex | **Trivial**. Add `supports_interactive_inline_closure: bool` to `ProviderInfo`. |
| `stdout_noise_prefixes` | Gemini | **Low effort**. Add `stdout_noise_prefixes` and `stderr_noise_prefixes` to `ProviderInfo`. |
| `stderr_noise_prefixes` | Codex, Gemini, OpenCode | **Low effort**. Same as above. |
| `apply_model` | Goose, OpenCode | **Medium effort**. Add `model_cli_flag: Option<&'static str>` (see Gap 3 above). |
| `apply_system_prompt` | Claude, Codex, Gemini, Goose, Kimi, OpenCode, Qwen | **Medium effort**. Most providers could be covered by extending `SystemPromptSpec` with flag templates. |
| `apply_structured_stream` | Claude, Codex, Gemini, Kimi, OpenCode, Qwen | **Medium effort**. Could be driven by a `structured_stream_flag: Option<&'static str>` catalog field. |
| `apply_non_interactive_flags` | Gemini, Qwen | **Low effort**. Flags that conflict with non-interactive mode could be listed in `ProviderInfo`. |
| `validate_non_interactive_requirements` | (none today) | N/A |
| `prepare_captured_output` / `parse_captured_output` | Gemini | **Low effort**. Captured-output format could be a catalog field. |
| `suppress_structured_stderr_on_success` | Gemini | **Low effort**. A bool in `ProviderInfo`. |
| `allowed_env_keys` | Codex, Gemini, Kimi, OpenCode, Qwen | **Low effort**. A static slice in `ProviderInfo`. |

## Ensuring a Single Source of Truth

### What We Do Today

#### Exhaustive Invariant Tests

The test suite in `lib/src/provider/tests.rs` enforces four categories of invariants:

1. **Registry completeness**: Every `Provider` variant resolves to a `ProviderInfo` whose `provider` field matches the lookup key. The `OnceLock`-backed registry array has exactly `PROVIDER_COUNT` slots.

2. **Non-empty mandatory fields**: `display_name`, `slug`, `binary`, `agent_offset`, `cli_aliases`, `docs_url` must all be non-empty. The `agent_offset` must start with `.`. Behavior trait objects must be non-null.

3. **Legacy facade agreement**: `agent_capabilities_facade_matches_catalog` asserts the legacy `AgentCapabilities` tree returned by `agent_for(provider)` is identical to `provider_info(provider).agent_capabilities()`. `resource_support_facade_matches_catalog` asserts the same for resource support. These tests catch drift between the two systems.

4. **Structural invariants**: Every provider declares at least one config path. Supported events have non-empty native names. Hook events imply `configurator.hooks_supported()`. Stream providers expose at least one event. ACP events imply non-`NotSupported` ACP support.

#### Source-Scan Drift Guard

The `no_unauthorized_match_provider_in_lib` test walks every `.rs` file in the lib crate, strips comments, and scans for three patterns:

1. `match <ident> { ... Provider::<Variant> => ... }` — match-form dispatch on Provider
2. `Provider::<Variant> => ` — standalone match arms
3. `[(Provider::<Variant>, ...)]` — provider tuple arrays (the duplicated-fact pattern)

An allow-list of six files is maintained: the central registry, identity helpers, the test file itself, adapter test fixtures, methods test fixtures, and the stream parser factory. Any new `match Provider` outside these files fails CI.

#### WrapperProfile Trait (CLI Layer)

In the CLI crate, `WrapperProfile` provides default implementations that derive from the central catalog: `binary()`, `agent_env()`, `apply_yolo()`, `apply_entrypoint()`, `apply_output_format()`, `prompt_arg_conventions()`, `supports_structured_stream()`, `stream_protocol()`, and `has_supported_yolo()` all read from `provider_info(self.provider())`. Providers only override when the default is insufficient.

#### Legacy Compatibility Bridge

`ProviderInfo` implements the `agents::Agent` trait (`fn id()` and `fn capabilities()`), so the legacy `agents::agent_for(provider)` registry forwards directly to `provider_info(provider)` without maintaining separate per-provider facade structs.

### How to Improve Conformance

#### 1. Extend the Drift Guard to the CLI Crate

**What**: The `no_unauthorized_match_provider_in_lib` scan currently covers only `claudine/lib/src/`. The CLI crate (`claudine/cli/src/`) has 296+ references to `Provider::<Variant>`, many of which are test fixtures, but several represent genuine provider-specific dispatch outside the metadata system (e.g. `composition/mod.rs` line 818 matching `Provider::Codex | Provider::Gemini` for MCP tag stripping, `wrap/mod.rs` line 541 checking `Provider::OpenCode` for env sanitization).

**Value**: A parallel drift guard for the CLI crate would prevent new provider-specific dispatch from being added without an explicit allow-list entry, forcing authors to add metadata instead.

**Current pain**: The CLI can freely grow `match Provider` blocks that bypass the catalog, creating silent drift between catalog data and actual behavior.

**Blocking factor**: The CLI's `WrapperProfile` trait intentionally allows per-provider overrides for complex behavior that doesn't fit the catalog (prompt delivery, resume args, sandbox). A drift guard would need a larger allow-list and more nuanced pattern matching to avoid false positives on legitimate trait-impl dispatch.

**Effort**: Medium. Clone the drift guard pattern, curate the allow-list for the CLI crate, integrate into CI.

#### 2. Migrate WrapperProfile Defaults to Catalog-Driven

**What**: Several `WrapperProfile` methods still require per-provider overrides despite being conceptually static facts: `prompt_delivery`, `build_resume_args`, `apply_sandbox`, `supports_interactive_inline_closure`. As described in the gaps section above, each of these could be replaced by a typed descriptor on `ProviderInfo` with a default `WrapperProfile` implementation that reads from it.

**Value**: The goal is that adding a new provider requires only a new `ProviderInfo` constant plus behavior trait implementations, with no CLI wrapper profile changes for common cases.

**Current pain**: Today, adding a provider requires changes in at least 4-6 files: the provider module (catalog data), the wrapper profile (CLI behavior), the stream parser factory, and the adapter module. The wrapper profile changes are the most complex and error-prone because they encode runtime behavior in imperative code rather than declarative data.

**Blocking factor**: `prompt_delivery` is the highest-value but highest-effort migration. The delivery mechanism interacts with entrypoints, mode switching, prompt flags, and the wire-RPC protocol. A full abstraction that handles all eight providers' delivery patterns is a significant design effort.

**Effort**: High for prompt delivery. Low-to-medium for the other methods.

#### 3. Deprecate and Remove the Legacy AgentCapabilities Tree

**What**: Today, every provider maintains **both** a typed `ProviderInfo` (36 fields) **and** a legacy `AgentCapabilities` tree (~80 fields across 15 nested structs). Tests enforce that they agree, but the duplication is a maintenance burden and a source of potential drift.

**Value**: Removing `AgentCapabilities` eliminates an entire parallel data surface that must be kept in sync. It also removes the `LazyLock` indirection and the fn-pointer accessor pattern.

**Current pain**: Every new metadata field must be added in two places and tested for agreement. The legacy tree uses string-heavy, loosely typed fields (`Vec<&'static str>`) where the typed catalog uses structured enums.

**Blocking factor**: Downstream consumers (linking layer, config discovery, reporting) still read from the legacy surface. Each consumer must be migrated to read from the typed catalog before the legacy tree can be removed.

**Effort**: High. Requires systematically migrating every consumer of `AgentCapabilities` fields to the typed `ProviderInfo` equivalents. The test suite already validates agreement, so the migration can proceed incrementally.

#### 4. Add a Compile-Time Exhaustiveness Check for Provider Coverage

**What**: The drift guard is regex-based and runs at test time. A `build.rs` script or a macro could verify at compile time that every `Provider` variant has a registry entry, every declared `EventMappingTable` covers a minimum set of events, and every provider with a non-`None` `stream_protocol` also has a `create_semantic_parser` implementation.

**Value**: Compile-time checks catch gaps before tests run and provide better error messages than test assertions.

**Current pain**: The `PROVIDER_COUNT` constant and `repr(usize)` indexing create implicit compile-time coupling, but there is no explicit compile-time assertion that the registry array covers every variant.

**Blocking factor**: The current `OnceLock` pattern and `repr(usize)` indexing already create a compile-time linkage (a missing slot causes an out-of-bounds panic at init time). A build script would need to parse Rust source to add meaningful checks beyond what the type system already provides.

**Effort**: Low-to-medium for basic exhaustiveness checks. Higher for deeper structural validation.

#### 5. Provider Metadata Schema Versioning

**What**: The JSON describe surface (`claudine providers --describe --format json`) is unversioned. If metadata fields are added, renamed, or removed, downstream consumers have no way to detect schema changes.

**Value**: A schema version field on `ProviderInfo` would let consumers adapt to changes without breaking.

**Current pain**: Any change to the serialized shape of `ProviderInfo` is an implicit breaking change for JSON consumers.

**Blocking factor**: There is only one known consumer today (the `--describe` CLI command), so the urgency is low. However, as the catalog becomes the foundation for more features (e.g. the composition layer's provider selection), the risk grows.

**Effort**: Trivial. Add `schema_version: u32` to `ProviderInfo` and include it in the serialized output.
