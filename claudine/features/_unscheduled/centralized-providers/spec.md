# Centralizing Providers in the Source Code

## Overview

Today, "everything we know about a provider" is split across at least seven modules — `events::provider`, `agents::*`, `linking::capabilities`, `stream::{mod, protocol::*}`, `mcp::{import, inject, state}`, `model_catalog::provider_sources`, and `cli/.../wrap/profile`. Each module owns one slice of provider truth, and each slice uses a different shape: enums on `Provider`, descriptive `&'static str` fields on `AgentCapabilities`, runtime trait impls on `WrapperProfile`, hand-rolled `match` arms on free functions, ad-hoc string lists in pre-parser tables, and so on.

This split is intentional in spirit (concerns evolve independently) but accidental in practice: there is no compile-time guarantee that a new `Provider` variant has been wired through every module, no typed bridge between the descriptive catalog and the runtime wrapper, no executable handle on the `LoggingCapabilities` strings, and no first-class home for ACP. Adding a ninth provider currently means visiting at least a dozen files armed with a checklist, and the only safety net is a handful of matrix tests that catch some-but-not-all omissions.

This spec proposes a **centralized provider model** that becomes the single source of truth for every per-provider fact, and a **strongly typed surface** that replaces the descriptive `&'static str` collections with parsed, executable types. The goal is not to collapse all behavior into one struct (that would defeat the original separation of concerns) — it is to (a) make every provider variant carry a typed handle to every facet of its behavior, and (b) make it impossible to add a `Provider` variant that compiles without addressing every facet.

### Decision Log

Three decisions made during clarification review (resolving Open Questions 1, 2, 3, and 6):

1. **Hybrid data + behavior split.** `provider_info(Provider) -> &'static ProviderInfo` returns a `Serialize`-derived data struct carrying every static field; a small `ProviderBehavior` trait, reached via `info.behavior`, holds the genuinely dynamic operations (`detect_from_payload`, `create_semantic_parser`). Load-bearing reason: structural serde round-trip + inspectability.
2. **`AgentId` collapsed in phase 0.** `AgentId` merges into `Provider` as a standalone, mechanically-revertable PR before any trait scaffolding lands. `AgentId` survives one release cycle as a `#[deprecated]` re-export. Load-bearing reason: de-risks phase 1 by separating identifier unification from trait scaffolding.
3. **`WrapperProfile` stays CLI-only.** The lib-side `ProviderInfo` carries no `wrapper_profile` accessor; the CLI keeps a parallel `wrapper_for(Provider)` registry. Load-bearing reason: forced by the lib/CLI crate-graph constraint — the original spec implicitly assumed a circular dependency that does not compile.

## Motivation

The "Future Improvements to Metadata" section of [`building-an-agent-wrapper.md`](../../topics/building-an-agent-wrapper.md) enumerates ten concrete gaps. They cluster into four themes:

1. **No central registry** — drift between `Provider`, `AgentId`, `AiCli`, `WrapperProfile`, `ProviderCapabilities`, `stream_protocol_for`, `provider_sources::*`, `argv::COMPOSITION_FLAGS_WITH_VALUE`, and `mcp::*`.
2. **Stringly-typed metadata** — `entrypoints: Vec<&'static str> = vec!["codex exec"]`, `output_formats: vec!["jsonl (--json)"]`, `gaps: Vec<&'static str>`, `LoggingCapabilities.session_locations` as glob-like prose.
3. **Duplicated knowledge** — `PromptArgConventions` overlaps `NonInteractiveCapabilities.entrypoints`; `WrapperProfile::apply_output_format` overlaps `NonInteractiveCapabilities.output_formats`; `Provider::native_event_name` overlaps the `SharedNativeEventMapping` tables that already exist for three providers.
4. **No first-class ACP** — ACP appears only in prose comments (`Goose request_permission`, `KimiCode --wire`); there is no `EventSupportLevel::Acp`, no `AcpCapabilities`, no `acp::adapter` trait surface.

## Goals

- Establish a `Provider` enum variant as the irrevocable identifier and hang every other piece of provider truth off it via typed associations.
- Replace every `Vec<&'static str>` that is actually structured data with a parsed type (enums, structs, typed paths, typed templates).
- Guarantee at compile time that every `Provider` variant has an entry in every cross-cutting registry (events, capabilities, streams, MCP, model catalog, wrappers, sniff binding).
- Eliminate the descriptive-vs-runtime split where the same fact is encoded twice (`AgentCapabilities` strings vs `WrapperProfile` runtime methods).
- Add first-class ACP support as a typed capability without churning unrelated providers.
- Preserve the modular layout — concerns still evolve independently — but make the seam between modules typed instead of nominal.
- Maintain backwards compatibility with all existing `claudine providers`, `claudine hooks --support`, `claudine hooks --mapping`, `claudine hooks --describe`, and `claudine init` output. The CLI surface should not regress.

## Non-Goals

- Collapsing the nine current source modules into one mega-file. The split between `events/`, `agents/`, `stream/`, `linking/`, `mcp/`, `model_catalog/`, and `cli/.../wrap/` stays.
- Replacing the `WrapperProfile` trait with a fully data-driven engine. Some provider behaviors are too quirky for declarative encoding (Kimi's temp-file YAML, Gemini's shadow `HOME` for `GEMINI.md`, OpenCode's mode-conditional YOLO). The trait stays, but it consumes typed catalog data instead of duplicating it.
- Auto-generating provider files from JSON/YAML manifests. We're moving `&'static str` to typed Rust, not Rust to TOML.
- Changing the user-facing Provider matching, fuzzy resolution, or composition resolver semantics. Those got a major redesign in 2026-04-25 and are working as designed.
- Adding new providers in the same change. This is pure refactor.

## Current State Inventory

The following table enumerates every module that knows something about each provider, what it knows, and the form it currently uses. **An "X" means the per-provider data is hard-coded in that module today; this spec proposes either removing it or backing it with the central catalog.**

| Module | What it knows | Current form |
|---|---|---|
| `lib/src/events/provider.rs::Provider` | Variant identifier, slug, display name, aliases, sniff binding, agent offset, docs URL, dashboard URL, skill support, fuzzy match, payload detection | Enum + per-method `match` arms (1474 LOC) |
| `lib/src/events/provider.rs::event_support_level` | Per-provider × per-event support level (Hook/NonHook/NotSupported) | Single giant `match` |
| `lib/src/events/provider.rs::native_event_name` | Per-provider × per-event native name | Mix of `SharedNativeEventMapping` table (3 providers) and giant fallback `match` (5 providers) |
| `lib/src/events/matrix.rs` | Tabular projections of the above | Pure derivation |
| `lib/src/agents/<provider>.rs` | `AgentCapabilities` (8 sub-structs, mostly `Vec<&'static str>`) | One file per provider, hand-written constants |
| `lib/src/agents/registry.rs` | `AgentId` → `&'static dyn Agent` | `OnceLock`-backed dispatch |
| `lib/src/adapters/<provider>.rs` | Inbound payload → `AgenticEvent` | Trait impl |
| `lib/src/config/<provider>.rs` | Hook registration (`AgentConfigurator`) | Trait impl |
| `lib/src/linking/capabilities.rs::ProviderCapabilities` | `ResourceSupport` per `LinkableResource`, `SkillFrontmatter` | One `*_capabilities()` function per provider |
| `lib/src/stream/mod.rs::stream_protocol_for` | Stream protocol per provider | `match` |
| `lib/src/stream/mod.rs::create_semantic_parser` | Constructor dispatch | `match` |
| `lib/src/stream/protocol/<provider>.rs` | Typed event enums | One file per provider |
| `lib/src/stream/<provider>_semantic.rs` | Typed → semantic projection | One file per provider |
| `lib/src/mcp/import.rs` | MCP import logic | Per-provider arm |
| `lib/src/mcp/state.rs` | MCP state tracking | Per-provider arm |
| `lib/src/mcp/inject.rs` | Runtime injection (3 providers only) | Per-provider arm |
| `lib/src/model_catalog/provider_sources.rs` | Static vs dynamic catalog source | `match` |
| `cli/src/commands/wrap/profile.rs::WrapperProfile` | Wrapper behavior (24 trait methods) | One static unit struct per provider (3057 LOC, 7 of 8 providers) |
| `cli/src/commands/wrap/profile.rs::profile_for_provider` | Provider → profile dispatch | `match` |
| `cli/src/argv.rs::COMPOSITION_FLAGS_WITH_VALUE` | Composition flag surface | Hand-maintained `&[&str]` (drift-tested) |
| `cli/src/argv.rs::Rule 1` | `--<provider>` boolean → slug | `match` |
| `claudine/docs/research/hooks/<provider>.md` | Hook research | One file per provider |
| `claudine/docs/research/cross-referencing/<provider>.md` | Resource portability research | One file per provider |
| External: `sniff::programs::AiCli` | Install detection enum | External crate |

Twenty-one in-tree dispatch points, plus one external dependency. A `Provider` variant compiles after touching three of them (`Provider`, `Display`, `PROVIDERS_DISPLAY_ORDER`), at which point matrix tests start failing for the rest. Not every miss is caught by tests today.

## Proposed Architecture

### Central type: `ProviderInfo` (data) + `ProviderBehavior` (trait)

Introduce a new top-level `claudine::provider` module containing a partitioned design: a public **data struct** carrying every static fact about a provider, and a small **behavior trait** for the genuinely dynamic operations. The two are linked by a `&'static dyn ProviderBehavior` field on the struct, so a single registry lookup returns both halves.

The partition exists for two load-bearing reasons:

1. **Serde round-trip is structural.** `ProviderInfo` derives `Serialize`, so the success criterion that `claudine providers --describe --format json` round-trips through serde without information loss is satisfied trivially by the data shape — no manual `Serialize` impl required for a 17-method trait.
2. **Inspectability.** All static fields are reachable as plain struct accesses (e.g. in tests, debuggers, snapshot output) without going through trait-object indirection.

Trait-object v-table cost was a secondary consideration — these are config-time lookups and the cost is negligible either way.

```rust
/// All static, serializable facts about a provider.
///
/// Populated once per provider variant as a `&'static ProviderInfo`. The
/// `behavior` field carries the small set of genuinely dynamic operations
/// (parser construction, payload detection) so a single lookup returns
/// both halves of the catalog.
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    // ----- Identity -----------------------------------------------------------
    pub provider: Provider,
    pub sniff_binding: AiCli,                     // typed bridge — replaces sniff_ai_cli()
    pub display_name: &'static str,               // "Kimi Code"
    pub slug: &'static str,                       // "kimi_code"
    pub binary: &'static str,                     // "kimi"
    pub agent_offset: &'static str,               // ".kimi"
    pub cli_aliases: &'static [&'static str],

    // ----- Documentation links -----------------------------------------------
    pub docs: &'static ProviderDocs,
    pub dashboards: &'static ProviderDashboards,

    // ----- Capability catalog (replaces today's AgentCapabilities) ------------
    pub capabilities: &'static ProviderCapabilities,

    // ----- Event mapping (data half; detection lives on the behavior trait) ---
    pub event_mapping: &'static EventMappingTable,

    // ----- Resource portability (today's linking::capabilities) ---------------
    pub resource_support: &'static ResourcePortability,

    // ----- Stream parsing (protocol selector is data; parser ctor is behavior) -
    pub stream_protocol: Option<StreamProtocol>,

    // ----- MCP support --------------------------------------------------------
    pub mcp: &'static McpSupport,

    // ----- Model catalog ------------------------------------------------------
    pub model_catalog_source: ModelCatalogSource,

    // ----- Behavior hook ------------------------------------------------------
    /// Skipped during serialization; the behavior trait is not data.
    #[serde(skip)]
    pub behavior: &'static dyn ProviderBehavior,
}

/// Genuinely dynamic per-provider operations.
///
/// Anything that takes runtime arguments or returns owned values lives here.
/// Everything else lives on `ProviderInfo` as a static field.
pub trait ProviderBehavior: Send + Sync + std::fmt::Debug + 'static {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool;

    fn create_semantic_parser(
        &self,
        sink: Box<dyn SemanticEventSink>,
        config: ParserConfig,
    ) -> Option<Box<dyn SemanticStreamParser>>;
}
```

Each provider gets exactly one `ProviderInfo` constant (e.g. `CLAUDE_INFO`, `CODEX_INFO`) and one zero-sized behavior implementor (e.g. `ClaudeBehavior`, `CodexBehavior`). Today's `agents/<provider>.rs` files become `provider/<provider>.rs` and grow to absorb the responsibilities listed above. The pair acts as a typed funnel: every cross-cutting query routes through a single `&'static ProviderInfo` lookup keyed on the `Provider` enum, with dynamic hooks reached via `info.behavior`.

Note that `ProviderInfo` carries no `agent_id()` accessor: `Provider` is the only identifier post phase 0 (see Migration Plan).

The lib-side `ProviderInfo` deliberately carries no `wrapper_profile` field. `WrapperProfile` is CLI-only (it depends on `tempfile`, `std::process::Command`, and `Path`-based filesystem fixtures) and the lib crate must not depend on the CLI crate. The CLI keeps a parallel `wrapper_for(Provider)` registry that consumes lib-side catalog data but is not reachable from lib. See "Migration of `WrapperProfile`" below.

### Central registry: `provider_info(Provider) -> &'static ProviderInfo`

Replaces (or absorbs) the existing:

- `agents::registry::agent_for(AgentId)` (after phase 0 collapses `AgentId` into `Provider`)
- `linking::capabilities::capabilities_for(Provider)`
- `stream::stream_protocol_for(Provider)`
- `stream::create_semantic_parser`
- `model_catalog::provider_sources::*`

Implementation uses `OnceLock` and an exhaustive `match` over `Provider`. The match is the **only** allowed `match` over `Provider` in the lib crate post-migration; everything else dispatches via the struct's fields or the `behavior` trait object.

A drift-detection test enumerates `PROVIDERS_DISPLAY_ORDER` and asserts `provider_info(p).provider == p` for each — proving every variant is registered and self-consistent.

`wrap::profile_for_provider(Provider)` is **not** absorbed into `provider_info`. It remains in the CLI crate as `wrapper_for(Provider) -> &'static dyn WrapperProfile`, with the same exhaustiveness invariant (`for p in PROVIDERS_DISPLAY_ORDER { assert_eq!(wrapper_for(p).provider(), p); }`). This split is forced by the lib/CLI crate-graph constraint, not preference.

### Strong typing replacements

#### 1. Path and template strings

Today: `LoggingCapabilities.session_locations: Vec<&'static str> = vec!["~/.claude/projects/<encoded-directory>/<session-uuid>.jsonl"]`.

Proposed:

```rust
pub enum PathTemplate {
    Static(PathBuf),
    Templated {
        segments: Vec<PathSegment>,
        // Resolved against a typed PathContext (home, repo_root, session_id, ...).
    },
}

pub enum PathSegment {
    Literal(&'static str),
    HomeDir,                // expands $HOME
    RepoRoot,               // expands repo root
    EncodedCwd,             // Claude's percent-style encoding of CWD
    SessionId,              // UUID slot
    DateYYYYMMDD,           // for Codex sessions/YYYY/MM/DD layout
    Glob(GlobKind),         // typed glob (single-component vs recursive)
}

pub fn resolve(&self, ctx: &PathContext) -> PathBuf;
```

The same type backs `agents::*::session_locations`, `agents::*::log_locations`, `mcp::*::config_paths`, and the new `ProviderInfo::dashboards` field. `ConfigCapabilities::user_files`, `project_files`, and `local_files` migrate from `Vec<PathBuf>` (already typed) to `Vec<PathTemplate>` so home expansion is consistent.

#### 2. Output formats

Today: `output_formats: Vec<&'static str> = vec!["jsonl (--json)"]`.

Proposed:

```rust
pub struct OutputFormatSupport {
    pub format: OutputFormat,           // existing universal enum
    pub native_name: &'static str,      // e.g. "stream-json"
    pub cli_flag: Option<&'static str>, // e.g. Some("--output-format")
    pub stdin_supported: bool,
}

pub struct NonInteractiveCapabilities {
    pub supported: bool,
    pub entrypoints: Vec<EntrypointSpec>,
    pub stdin_supported: bool,
    pub output_formats: Vec<OutputFormatSupport>,  // typed
    pub structured_output_supported: bool,
    pub resume_supported: bool,
    pub limitations: Vec<KnownLimitation>,
}

pub struct EntrypointSpec {
    pub subcommand: Option<&'static str>,    // "exec", "run", or None
    pub required_flags: &'static [&'static str], // ["--print"]
    pub mode: EntrypointMode,                // NonInteractive, Interactive, Both
}
```

The wrapper profile's `apply_output_format`, `apply_entrypoint`, and `apply_non_interactive_flags` consume these typed specs directly, so the descriptive catalog and the runtime mapping are the same source of truth.

#### 3. System-prompt delivery

Today: `SystemPromptCapabilities` has prose strings (`replacement_mechanisms: vec!["--system-prompt", "--system-prompt-file"]`) and `WrapperProfile::apply_system_prompt` reimplements the same knowledge as runtime args.

Proposed:

```rust
pub enum SystemPromptDelivery {
    /// Direct flag carrying the prompt text inline.
    InlineFlag { flag: &'static str },
    /// Flag carrying a file path; file is created at runtime.
    FileFlag { flag: &'static str },
    /// Env var carrying a path to a temp file (e.g. GEMINI_SYSTEM_MD).
    EnvVarFile { env_var: &'static str },
    /// Shadow HOME with a synthesized memory file (e.g. .gemini/GEMINI.md).
    ShadowHomeFile { relative_path: PathBuf },
    /// Provider-specific composition required (e.g. Kimi --agent-file YAML).
    Custom(SystemPromptCustomTag),
    /// Not supported in this mode.
    Unsupported,
}

pub struct SystemPromptCapabilities {
    pub append: SystemPromptDeliveryByMode,
    pub replace: SystemPromptDeliveryByMode,
    pub memory_files: Vec<PathTemplate>,
}

pub struct SystemPromptDeliveryByMode {
    pub interactive: SystemPromptDelivery,
    pub non_interactive: SystemPromptDelivery,
}
```

The `Custom(SystemPromptCustomTag)` variant is an explicit escape hatch so Kimi's "write `.md` body + write `.yml` agent + pass `--agent-file`" stays in code, but the *fact* that it requires custom handling is typed.

#### 4. YOLO

Today: `PermissionCapabilities.yolo_equivalent: Option<&'static str>` and `WrapperProfile::apply_yolo` reimplement the mapping.

Proposed:

```rust
pub enum YoloSupport {
    None,                                                    // OpenCode interactive
    DirectFlag { native_flag: &'static str },                // Claude
    DirectFlagWithAlias { native_flag: &'static str, aliases: &'static [&'static str] }, // Codex
    NonInteractiveOnly { non_interactive_flag: &'static str }, // OpenCode
    EnvVar { env_var: &'static str, value: &'static str },   // hypothetical future
}
```

`WrapperProfile::apply_yolo` becomes a default impl that consumes `capabilities.permissions.yolo`. A handful of providers will still override `apply_yolo_for_mode` for genuinely conditional behavior.

#### 5. Reasoning levels

Today: `ReasoningCapabilities { style: ReasoningStyle, levels_or_controls: Vec<&'static str>, notes: Vec<&'static str> }`.

Proposed:

```rust
pub enum ReasoningSupport {
    NotSupported,
    NotDocumented,
    NamedLevels { flag: &'static str, levels: &'static [&'static str] },
    NumericBudget { flag: &'static str, min: u32, max: u32, default: Option<u32> },
    BinaryToggle { flag: &'static str, on: &'static str, off: &'static str },
    ProviderSpecific(ReasoningCustomTag),
}
```

The wrapper can derive `--reasoning-effort` argv from this without a per-provider runtime helper.

#### 6. Event mapping

Today: split between `Provider::event_support_level` (one giant match), `Provider::native_event_name` (another match with three providers using `SharedNativeEventMapping` tables), and `adapters/<provider>.rs` parsers.

Proposed:

```rust
pub struct EventMappingTable {
    pub support: &'static [(AgenticEvent, EventSupportLevel)],
    pub mappings: &'static [EventMapping],
}

pub struct EventMapping {
    pub event: AgenticEvent,
    pub native_name: &'static str,
    pub parse_aliases: &'static [&'static str],
    pub support: EventSupportLevel,
    pub capture_method: CaptureMethod,   // Hook | StreamParse | WireProxy | Acp
}

pub enum CaptureMethod {
    NativeHook,
    StreamParse(StreamProtocol),
    WireProxy(WireProxyMode),
    Acp(AcpEvent),
    Wrapper,
    None,
}
```

`event_support_level` and `native_event_name` become trivial lookups. `EventSupportLevel` may absorb `CaptureMethod` directly (Open Question).

#### 7. ACP first-class support

Today: zero. Goose and Kimi mention ACP only in comments.

Proposed:

```rust
pub struct AcpSupport {
    pub server_mode: AcpServerMode,
    pub client_supported: bool,
    pub events_via_acp: &'static [AcpEvent],
}

pub enum AcpServerMode {
    NotSupported,
    Native,                          // first-class ACP server
    AvailableViaWireProxy,           // Kimi --wire
}

pub enum AcpEvent {
    RequestPermission,               // Goose request_permission
    ApprovalRequest,                 // Kimi ApprovalRequest
    ToolCall,
    ToolResult,
    SessionUpdate,
    Custom(&'static str),
}
```

Slot it into `RuntimeCapabilities` alongside the existing seven sub-structs. `EventSupportLevel` adds an `Acp` variant. The `acp` skill can backfill detail.

#### 8. Confidence and gaps

Today: `ConfidenceProfile.gaps: Vec<&'static str>` of free-form sentences.

Proposed:

```rust
pub struct ConfidenceProfile {
    pub overall: Confidence,
    pub by_area: &'static [AreaConfidence],
    pub gaps: &'static [KnownGap],
}

pub struct KnownGap {
    pub area: KnownGapArea,
    pub note: &'static str,
    pub tracker: Option<&'static str>,   // e.g. "claudine/docs/research/cross-referencing/claude-code.md"
}

pub enum KnownGapArea {
    Skills,
    SlashCommands,
    Subagents,
    Scripts,
    Hooks,
    Stream,
    Mcp,
    Acp,
    SystemPrompt,
    Permissions,
    Reasoning,
    Logging,
    Billing,
    Other,
}
```

Now `claudine providers --gaps` (new flag) can group by `KnownGapArea` instead of dumping prose.

#### 9. PromptArgConventions consolidation

Today: `cli/.../profile.rs::PromptArgConventions` is a CLI-only struct that overlaps `NonInteractiveCapabilities.entrypoints`.

Proposed: promote `PromptArgConventions` into the lib crate as part of `NonInteractiveCapabilities`, derived from the new `EntrypointSpec` plus a typed `value_taking_flags` registry. The wrapper profile reads it via `provider.capabilities().non_interactive.prompt_arg_conventions()`.

#### 10. Composition flags surface

Today: `argv::COMPOSITION_FLAGS_WITH_VALUE` is a hand-maintained `&[&str]` policed by a drift test that enumerates clap's `augment_args`.

Proposed: have clap emit the surface at build time via a `clap::CommandFactory`-driven static. The drift test becomes assertion-free (the surface *is* clap's surface).

### Migration of `WrapperProfile`

`WrapperProfile` stays as a **CLI-only trait** with a **CLI-only registry**. It depends on `tempfile`, `std::process`, `Path`-based filesystem fixtures, etc., none of which the lib crate carries. Crucially, the lib-side `ProviderInfo` does **not** carry a `wrapper_profile()` accessor: doing so would require lib to depend on CLI types, a circular dependency.

Instead, the CLI crate maintains its own parallel registry:

```rust
// cli/src/commands/wrap/profile.rs
pub fn wrapper_for(p: Provider) -> &'static dyn WrapperProfile {
    match p { /* one arm per variant, OnceLock-backed */ }
}
```

Each `WrapperProfile` implementor consumes the lib-side catalog by calling `provider_info(self.provider())` and reading the relevant fields. Default trait implementations consume the catalog:

- `apply_yolo` derives from `provider_info(p).capabilities.permissions.yolo`.
- `apply_output_format` derives from `provider_info(p).capabilities.non_interactive.output_formats`.
- `apply_entrypoint` derives from `provider_info(p).capabilities.non_interactive.entrypoints`.
- `apply_model` derives from `provider_info(p).capabilities.model.cli_flags`.
- `prompt_arg_conventions` derives from `provider_info(p).capabilities.non_interactive`.
- `stream_protocol`, `supports_structured_stream`, `supports_resume` all become catalog reads.

Wrappers retain freedom to override for irreducible quirks (Kimi's `--agent-file`, OpenCode's mode-conditional YOLO, Codex's `model_instructions_file` config-merge). The override surface shrinks substantially from today's ~24 trait methods × 7 implementors.

The CLI-side `wrapper_for` registry has its own exhaustiveness test; see "Exhaustiveness Tests" below.

### What stays per-provider

Some logic is genuinely irreducible and stays in per-provider files:

- Stream protocol modules (`stream/protocol/<provider>.rs`) — typed event enums per stream format.
- Stream semantic parsers (`stream/<provider>_semantic.rs`) — `serde_json::Value` → `SemanticEvent` projection.
- Event adapters (`adapters/<provider>.rs`) — inbound-payload parser.
- Hook configurators (`config/<provider>.rs`) — config-file write logic with format-specific (JSON/JSONC/TOML/YAML) mutation.
- MCP import/export logic (`mcp/import.rs`, `mcp/export.rs`) — provider-specific JSON/TOML round trips.
- MCP runtime injection (`mcp/inject.rs`) — shadow `HOME` synthesis (Codex, Gemini) vs env-var content (OpenCode).
- Wrapper overrides for genuine quirks (small subset of `WrapperProfile`).

Each of these is reachable through `provider_info(Provider).<field>` (or `provider_info(Provider).behavior.<method>(...)` for dynamic operations) so callers never `match` over `Provider`; only the central registry does. The CLI-side `wrapper_for(Provider)` registry is the one permitted parallel match.

### Exhaustiveness Tests

Once the central registry is in place, four test invariants pin everything:

1. **Registry exhaustiveness** — `for p in PROVIDERS_DISPLAY_ORDER { assert_eq!(provider_info(p).provider, p); }`. Catches missing arms in the central match.

2. **Cross-module exhaustiveness** — `for p in PROVIDERS_DISPLAY_ORDER { let info = provider_info(p); let _ = info.event_mapping; let _ = info.capabilities; let _ = info.resource_support; … }`. Touches every field (and exercises `info.behavior` via `detect_from_payload`/`create_semantic_parser` smoke calls) to force a panic-or-stub if any provider hasn't been wired through a facet.

3. **Sniff binding round-trip** — `for p in PROVIDERS_DISPLAY_ORDER { let cli: AiCli = provider_info(p).sniff_binding; assert_eq!(Provider::from_sniff(cli), Some(p)); }`. Catches drift with the external `sniff` crate.

4. **CLI-side wrapper exhaustiveness** — `for p in PROVIDERS_DISPLAY_ORDER { assert_eq!(wrapper_for(p).provider(), p); }`, located in the CLI crate. Mirrors invariant (1) on the parallel `wrapper_for` registry so the lib/CLI split cannot drift independently.

Plus the existing per-area matrix tests (event support, linking capabilities, stream protocols, argv flags) which now derive from the same source.

## Detailed Type Reference

Concretely, the new types live as follows:

```
claudine/lib/src/provider/
├── mod.rs              # ProviderInfo data struct, ProviderBehavior trait, provider_info() registry
├── identity.rs         # ProviderIdentity helper types (slug, display, aliases, sniff_binding, ...) — no AgentId field
├── docs.rs             # ProviderDocs, ProviderDashboards
├── capabilities.rs     # ProviderCapabilities, ConfigCapabilities (re-export from lib)
├── runtime.rs          # RuntimeCapabilities + sub-structs
├── path_template.rs    # PathTemplate, PathSegment, PathContext
├── event_mapping.rs    # EventMappingTable, EventMapping, CaptureMethod
├── acp.rs              # AcpSupport, AcpServerMode, AcpEvent
├── system_prompt.rs    # SystemPromptDelivery, SystemPromptDeliveryByMode
├── yolo.rs             # YoloSupport
├── reasoning.rs        # ReasoningSupport
├── output_format.rs    # OutputFormatSupport, EntrypointSpec, EntrypointMode
├── known_gap.rs        # KnownGap, KnownGapArea
├── claude.rs           # ClaudeProvider impl
├── codex.rs            # CodexProvider impl
├── gemini.rs           # GeminiProvider impl
├── goose.rs            # GooseProvider impl
├── kimi.rs             # KimiProvider impl
├── opencode.rs         # OpenCodeProvider impl
├── qwen.rs             # QwenProvider impl
├── roo.rs              # RooProvider impl
├── registry.rs         # provider_info(), all_providers()
└── tests.rs            # exhaustiveness invariants
```

The legacy paths (`agents/`, `linking/capabilities.rs`) remain as thin re-exports for two release cycles to ease external consumer migration, then are deleted.

## Migration Plan

The migration is necessarily large but can be staged incrementally, with each phase compiling, passing tests, and shippable on its own. None of the phases require synchronous changes to external consumers.

### Phase 0 — `AgentId` unification (standalone PR)

Collapse `AgentId` into `Provider` as an independent, mechanically-revertable PR before any new trait scaffolding lands. The load-bearing reason: separating identifier unification from trait scaffolding de-risks phase 1 and removes a class of cross-phase translation noise (`AgentId::from(provider)` / `provider.agent_id()`) that would otherwise infect every subsequent phase.

1. Rename `AgentId` to `Provider` (or merge into `Provider`) at every call site in `claudine/lib/src/agents/`.
2. Update `agents::registry::agent_for` to take `Provider`.
3. Keep `AgentId` as a `#[deprecated]` re-export of `Provider` for one release cycle so external consumers can update without breakage.
4. Verify all existing tests pass; no behavior change beyond the rename.

Deliverable: a single-enum world where `Provider` is the only identifier. No new module exists yet.

### Phase 1 — Skeleton + facade (no behavior change)

1. Create `claudine/lib/src/provider/` module with `ProviderInfo`, `ProviderBehavior`, identity types, and one stub `ProviderInfo` constant per provider whose data fields delegate to (or duplicate) values from the existing modules (`agent_for`, `capabilities_for`, etc.). The lib stub does **not** delegate to CLI types — there is no `wrapper_profile` field.
2. Add `provider_info(Provider) -> &'static ProviderInfo`.
3. Wire the four exhaustiveness tests above (the CLI-side wrapper exhaustiveness lives in the CLI crate; the other three live in lib).
4. Add `--describe` JSON output backed by `serde_json::to_string(&provider_info(p))` so a snapshot test can pin behavior across migrations.

Deliverable: new module exists, no callers, all tests pass. `Provider` is the only identifier.

### Phase 2 — Migrate `linking::capabilities` and `agents::registry`

1. Move `ProviderCapabilities` and `AgentCapabilities` data into `provider/<provider>.rs` files (one per provider).
2. Reroute `linking::capabilities::capabilities_for` and `agents::registry::agent_for` to `provider_info(p).capabilities` / `provider_info(p).resource_support`. (Both functions now take `Provider`, courtesy of phase 0.)
3. Delete `agents/<provider>.rs` and the per-`*_capabilities()` functions in `linking/capabilities.rs`.
4. Update matrix tests; snapshot output should be unchanged.

Deliverable: two of the eight modules consolidated; ~1000 LOC removed.

### Phase 3 — Migrate event mapping

1. Move `event_support_level` and `native_event_name` per-provider rows into the `event_mapping` field of each provider's `ProviderInfo` constant.
2. Delete `Provider::event_support_level`, `Provider::native_event_name`, `Provider::registration_native_event_name` (or thin them to forwarding).
3. Delete the per-provider `*_SHARED_NATIVE_MAPPINGS` constants (now folded into `EventMappingTable`).
4. Add `CaptureMethod` to `EventMapping` and wire `EventSupportLevel::Acp` for Goose `request_permission` and Kimi `ApprovalRequest`.

Deliverable: one giant file (`events/provider.rs`, currently 1474 LOC) shrinks dramatically; native-name research is now grep-able by provider file.

### Phase 4 — Migrate stream and MCP

1. `stream::stream_protocol_for(p)` → `provider_info(p).stream_protocol`.
2. `stream::create_semantic_parser(p, …)` → `provider_info(p).behavior.create_semantic_parser(…)`. Each provider's `ProviderBehavior` impl forwards to the existing `*_semantic.rs` constructor.
3. Same for `mcp::import::*`, `mcp::state::*`, `mcp::inject::*` — each `match` becomes a read of `provider_info(p).mcp.<field>` (or a call into a method on `McpSupport` that takes the runtime arguments).

Deliverable: `match Provider::… ` arms outside the lib-side central registry drop to zero. The CLI-side `wrapper_for` registry remains as the one permitted parallel match.

### Phase 5 — Strongly type the descriptive surface

1. Replace `Vec<&'static str>` log-paths with `Vec<PathTemplate>`. Add resolver. Update `claudine logs` and any reporting that previously didn't consume them.
2. Replace `output_formats: Vec<&'static str>` with `Vec<OutputFormatSupport>`. Update wrappers to derive `apply_output_format`.
3. Replace `entrypoints: Vec<&'static str>` with `Vec<EntrypointSpec>`. Update `apply_entrypoint`.
4. Replace `yolo_equivalent: Option<&'static str>` with `YoloSupport`. Update `apply_yolo`.
5. Replace `replacement_mechanisms` and similar with `SystemPromptDeliveryByMode`. Update `apply_system_prompt`.
6. Replace `gaps: Vec<&'static str>` with `Vec<KnownGap>`.

Deliverable: ten of the "Future Improvements" items are closed; wrapper profiles shrink.

### Phase 6 — Wrapper profile thinning

1. Convert `WrapperProfile` default impls to read from the catalog wherever possible.
2. Delete trivial per-provider overrides that now match the catalog-derived default.
3. Promote `PromptArgConventions` from CLI into `NonInteractiveCapabilities` and have `WrapperProfile::prompt_arg_conventions` default to the catalog reading.
4. Replace `argv::COMPOSITION_FLAGS_WITH_VALUE` with a clap-derived static.

Deliverable: `cli/src/commands/wrap/profile.rs` shrinks from 3057 LOC to roughly half; provider-specific quirks remain, scaffolding evaporates.

### Phase 7 — ACP scaffold

1. Add `EventSupportLevel::Acp`, `AcpSupport`, `AcpEvent`.
2. Wire Kimi `ApprovalRequest` and Goose `request_permission` to `EventSupportLevel::Acp` in the event mapping table.
3. Add `claudine hooks --capture-method` to surface the new metadata.

Deliverable: ACP becomes typed, not commentary.

### Phase 8 — Cleanup

1. Delete `agents/` legacy re-exports.
2. Delete `linking/capabilities.rs::*_capabilities()` re-exports.
3. Delete the `#[deprecated]` `AgentId` re-export introduced in phase 0 (one release cycle elapsed).
4. Delete dead `match Provider` arms.
5. Update `claudine/docs/topics/building-an-agent-wrapper.md` to reflect the new architecture; remove the "Future Improvements" section.

Deliverable: authoritative, drift-free provider system.

## Verification

Each phase ships independently with the following gates:

- `just test` for `claudine` — all matrix and exhaustiveness tests pass.
- `just lint` for `claudine` — no warnings.
- `just doctest` for `claudine`.
- Snapshot tests for `claudine providers`, `claudine hooks --support`, `claudine hooks --mapping`, `claudine hooks --describe` — output unchanged across phases 1–7. After phase 7 the snapshot picks up the new `Acp` capture method (intentional).
- Smoke test: `claudine claude`, `claudine codex`, `claudine gemini`, `claudine goose`, `claudine kimi`, `claudine opencode`, `claudine qwen` against trivial prompts — each wrapper still launches.
- Smoke test: `claudine init --quick` produces the same registration output as before.

After phase 5, add a property test that for every provider × every `AgenticEvent`:

- If `support.is_supported()`, `mapping.native_name` is non-empty.
- If `mapping.capture_method` is `Hook`, the configurator is registered.
- If `mapping.capture_method` is `StreamParse`, the stream protocol is `Some(_)`.
- If `mapping.capture_method` is `Acp`, `AcpSupport` is non-`NotSupported`.

## Open Questions

1. **Module home for the central trait.** **Resolved:** new `claudine::provider` top-level module (see "Decision Log" / Decision 2). `Provider` is re-exported from its current home for one release cycle.

2. **`AgentId` vs `Provider`.** **Resolved:** collapsed in phase 0 as a standalone, mechanically-revertable PR before any trait scaffolding lands (see Decision 2). `AgentId` survives one release cycle as a `#[deprecated]` re-export. Load-bearing reason: separating identifier unification from trait scaffolding de-risks phase 1 and removes cross-phase translation noise.

3. **Static dispatch vs trait objects.** **Resolved:** hybrid — `ProviderInfo` data struct + small `ProviderBehavior` trait (see Decision 1). The load-bearing reason was structural serde round-trip (a `Serialize` derive on the data struct trivially satisfies the round-trip success criterion) plus inspectability; v-table cost was secondary.

4. **External `sniff` crate dependency** — the `sniff_binding` field depends on the external `sniff::programs::AiCli` enum. If `sniff` ever drops or renames a variant, claudine breaks. Acceptable today but worth a "known external dep" doc note. Should the sniff binding stay implicit (current state — runtime panic) or become a `Result<AiCli>` so missing variants are surfaced as a typed error? Recommendation: typed `Result`.

5. **Backwards compatibility for external consumers** — `claudine::events::Provider`, `claudine::agents::*`, `claudine::linking::capabilities::*` are all public and may have external downstream consumers (the rusty-biscuit monorepo has none, but the API is exported). Phase 8 deletes the legacy re-exports; should we deprecate them for two cycles first? Recommendation: yes — `#[deprecated]` markers from phase 5 onwards, removal in a release after phase 8.

6. **Should `WrapperProfile` move to the lib crate?** **Resolved:** no — `WrapperProfile` stays CLI-only with a parallel CLI-side `wrapper_for(Provider)` registry (see Decision 3). The lib-side `ProviderInfo` does NOT carry a `wrapper_profile` accessor; doing so would require lib to depend on CLI types, a circular crate dependency that the original spec implicitly assumed away. The answer here was forced by the lib/CLI crate split, not preference.

7. **`KnownGap.tracker` field** — should this be a `PathBuf` (file ref) or a typed `GapTracker` enum (file, GitHub issue, comment, etc.)? Recommendation: start with `Option<&'static str>` and tighten in a future iteration when there are enough trackers to warrant a typed model.

## Risks and Mitigations

- **Risk: massive churn during migration.** Phases 1–4 each touch one cross-cutting module at a time and are individually shippable. Each phase has a clean rollback (revert that single PR).
- **Risk: snapshot test churn.** All output-affecting changes are in phase 7 (ACP) and phase 8 (cleanup); phases 1–6 maintain bit-for-bit output equivalence.
- **Risk: external consumers break on legacy re-export removal.** Phase 8 is deferred until external consumers are surveyed; deprecation warnings ship in phase 5.
- **Risk: `sniff` crate drift.** Phase 1's typed sniff binding round-trip test catches drift at compile time / test time, not at runtime.
- **Risk: trait-object overhead.** Negligible — and largely sidestepped by the hybrid design (most fields are direct struct accesses; only `info.behavior.<method>` goes through a v-table). These are config-time lookups, not hot-path. A flame graph from `claudine compose` would confirm < 0.01% wall-clock impact, well within noise.
- **Risk: ACP modeling is premature.** Phase 7 only types what is *already* in the codebase as comments (Goose `request_permission`, Kimi `ApprovalRequest`); it does not add new capture mechanisms. Future ACP work fills the typed slots.

## Success Criteria

- The number of `match` expressions over `Provider` in the lib crate outside `claudine::provider::registry` drops to **zero**. The CLI crate retains exactly one match (the `wrapper_for` registry).
- Adding a hypothetical ninth provider requires editing exactly **one** new lib file (`provider/<name>.rs`) plus extending the central lib registries (`Provider` enum, `PROVIDERS_DISPLAY_ORDER`, `provider_info`) and the CLI-side `wrapper_for` registry. All other failures (missing event mapping, missing capability, missing stream protocol, missing wrapper profile) become compile errors via exhaustiveness tests on each registry.
- `claudine providers --describe --format json` round-trips through serde without information loss. **Structurally guaranteed** by the `#[derive(Serialize)]` on `ProviderInfo` — no manual serialization plumbing needed.
- The "Future Improvements to Metadata" section in [`building-an-agent-wrapper.md`](../../topics/building-an-agent-wrapper.md) is replaced with a "Migration History" subsection.
- `cli/src/commands/wrap/profile.rs` LOC drops by at least 40%.
- `events/provider.rs` LOC drops by at least 50%.
- Every `Vec<&'static str>` field in `RuntimeCapabilities` is replaced by a typed equivalent except `notes` (which stays free-form by design).
