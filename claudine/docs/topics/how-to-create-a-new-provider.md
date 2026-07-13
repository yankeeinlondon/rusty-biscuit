---
categories:
    - technical
    - onboarding
    - provider-metadata
---

# How to Create a New Provider

## Overview

Adding a new agentic CLI provider to Claudine is a **research-heavy, code-light** process. The goal of the centralized provider catalog is that most provider-specific facts live as declarative data in a single `ProviderInfo` constant, not as scattered imperative code.

This document describes the complete end-to-end process: what to research, what metadata to collect, where to inject it, and how to generate the required code without building a dedicated code-generation library.

> **Philosophy**: Use template-based generation for the provider catalog file and simple file edits for the mechanical plumbing. A separate codegen library is overkill for a workflow that runs a few times per year.

---

## Phase 1: Research

Before writing any code, gather metadata about the new provider. The research prompt (`@claudine/prompts/new-provider.md`) fans out five parallel investigations:

| Research Area | Output File | What It Covers |
|---------------|-------------|----------------|
| **Basics** | `docs/research/basics/{file}.md` | Binary name, install methods, config paths, model selection, version flag behavior |
| **Agent Definitions** | `docs/research/agent-definitions/{file}.md` | Skills, commands, agents, slash commands, memory files, system prompt delivery |
| **Hooks** | `docs/research/hooks/{file}.md` | Hook event names, config file format, hook registration mechanism |
| **CLI Information** | `docs/research/usage/{file}.md` | Entrypoints, output formats, YOLO/auto-approve, reasoning controls, ACP support |
| **Usage Metrics** | `docs/research/usage/{file}.md` | Billing model, usage dashboard, stream protocol, session log paths |
| **Agent Errors** | `docs/research/agent-errors/{file}.md` | Ordered structured-stream error kinds/messages/codes with per-item provenance |

Each research file writes its structured findings into **Markdown frontmatter**. The frontmatter schema is the single source of truth for the next phase.

### Key Research Questions

- What is the primary binary name on PATH?
- Are there alternate binary names or aliases?
- How is it installed on macOS, Linux, and Windows?
- What config files does it use and in what format (JSON, TOML, YAML)?
- Does it support structured output streams (JSONL, NDJSON, etc.)?
- What hook events does it emit and how are they named?
- Does it support MCP, skills, or ACP?
- How does it accept prompts in interactive vs non-interactive mode?
- What CLI flags control YOLO/auto-approve, reasoning, and model selection?
- Which structured error discriminators, messages, and numeric codes are safe
  inputs to the existing case-insensitive substring classifier?

If the provider has a structured stream parser, author and validate its
`agent-errors` research document before wiring the parser. Do not add an
`ERROR_KEYWORDS` constant or an `error_vocabulary` facts key. Generation
projects the provenance-bearing research objects into
`lib/src/stream/providers/vocabulary.rs`; bucket and item order are runtime
precedence. A provider without a parser keeps an explicit empty runtime table
until parser onboarding makes the researched records executable and adds
classifier fixtures.

---

## Phase 2: Sniff Detection Metadata

Claudine delegates host detection to the `sniff` library. To make a new provider detectable, you must register it in sniff's `AiCli` catalog.

### What Sniff Needs

Sniff performs a simple PATH existence check using the `which` crate, plus optional version probing. At compile time it needs a `ProgramInfo` entry:

| Field | Source |
|-------|--------|
| `binary_name` | Primary executable on PATH (from Basics research) |
| `display_name` | Human-readable name |
| `description` | One-line description |
| `website` | Official documentation URL |
| `version_flag` | Usually `--version` (`VersionFlag::Long`) |
| `parse_strategy` | Usually `FirstLine` |
| `version_regex` | Only if version output needs regex extraction |
| `version_prefix` | Only if version line has a prefix to skip |
| `alternate_binary_names` | Aliases on PATH (e.g. `kimi-cli` for `kimi`) |
| `os_availability` | Usually `ALL_OS` |
| `repo` | Source repository URL |
| `installation_methods` | Brew, npm, cargo, apt, etc. (from Basics research) |
| `system_prerequisites` | Usually empty for AI CLIs |

### Files to Change in Sniff

1. **`sniff/lib/src/programs/enums/categories.rs`**
   - Add variant to `AiCli` enum
   - Variant order must match `AI_CLI_INFO` table index

2. **`sniff/lib/src/programs/enums/metadata.rs`**
   - Add installation methods static (e.g. `NEW_PROVIDER_INSTALL`)
   - Add `ProgramInfo` entry to `AI_CLI_INFO` table
   - Add `serde_key` match arm in `CategoryEnum` impl

3. **Tests are self-enforcing**
   - `test_ai_cli_count_matches_info`
   - `test_all_category_enums_cover_all_programs`
   - `test_program_mapping_is_bijective`
   - `test_category_variant_indices_are_contiguous`

### Files to Change in Claudine (Sniff Wiring)

1. **Provider module** (`claudine/lib/src/provider/{file}.rs`):
   ```rust
   sniff_binding: AiCli::NewProvider,
   ```

2. **Provider methods test** (`claudine/lib/src/provider/methods.rs`):
   ```rust
   assert_eq!(Provider::NewProvider.sniff_ai_cli(), AiCli::NewProvider);
   ```

---

## Phase 3: Provider Catalog Metadata

The heart of a new provider is its `ProviderInfo` constant in `claudine/lib/src/provider/{file}.rs`. This is a large static struct (~400–600 lines) that declares every static fact about the provider.

### Identity Fields

| Field | Example | Source |
|-------|---------|--------|
| `provider` | `Provider::Claude` | Enum variant (added in Phase 4) |
| `display_name` | `"Claude"` | Friendly name |
| `slug` | `"claude"` | Snake-case identifier for paths/JSON |
| `short_name` | `"claude"` | Abbreviated name for logs |
| `binary` | `"claude"` | Executable on PATH |
| `agent_offset` | `".claude"` | Dot-prefixed config directory |
| `cli_aliases` | `&["claude"]` | Accepted alias forms |
| `docs_url` | `"https://..."` | Documentation homepage |
| `usage_dashboard_url` | `Some("...")` or `None` | Billing dashboard |
| `sniff_binding` | `AiCli::Claude` | Sniff enum variant (from Phase 2) |
| `supports_skills` | `true` / `false` | Skill discovery support |

### Behavior Traits

Four trait objects on `ProviderInfo` carry dynamic behavior. Most providers only need to implement a subset:

| Trait | Override When | Typical Implementation |
|-------|---------------|------------------------|
| `ProviderBehavior` | Always (required: `detect_from_payload`) | Usually returns `false` unless the provider has a distinct raw hook payload shape |
| `McpBehavior` | Provider supports MCP | `supported() -> true`, plus `discover_configs`, `parse_config`, `native_config_path`, `read_existing_native_servers`, `write_native_config` |
| `AdapterBehavior` | Provider has inbound payload parsing | `provider_adapter()` returns a `ProviderAdapter` struct |
| `ConfiguratorBehavior` | Provider has config-file hooks | `hooks_supported() -> true`, `agent_configurator()` returns an `AgentConfigurator` |

### Typed Catalog Data

| Field | Type | What to Populate |
|-------|------|------------------|
| `stream_protocol` | `Option<StreamProtocol>` | `Some(StreamProtocol::Jsonl)`, `Ndjson`, `StreamJson`, `WireJsonRpc`, or `None` |
| `event_mapping` | `&'static EventMappingTable` | Per-event support level, native names, parse aliases |
| `session_log_paths` | `&'static [PathTemplate]` | JSONL transcript file templates |
| `config_paths` | `&'static [PathTemplate]` | Config file templates (first = primary user config) |
| `memory_files` | `&'static [PathTemplate]` | Files contributing to system prompt hierarchy |
| `output_formats` | `&'static [OutputFormatSupport]` | Non-interactive output formats with native flags |
| `entrypoints` | `&'static [EntrypointSpec]` | Subcommands and required flags per mode |
| `system_prompt` | `&'static SystemPromptSpec` | Append/replace delivery by interactive vs non-interactive |
| `yolo` | `YoloSupport` | `None`, `DirectFlag`, `DirectFlagWithAlias`, `NonInteractiveOnly`, `EnvVar` |
| `reasoning` | `ReasoningSupport` | `None`, `NamedLevels`, `NumericBudget`, `BinaryToggle`, `ProviderSpecific` |
| `known_gaps` | `&'static [KnownGap]` | Catalogued unknowns (can be empty) |
| `acp` | `AcpSupport` | ACP server mode, client capability, captured events |
| `prompt_arg_conventions` | `PromptArgConventions` | How the CLI represents a prompt on argv |
| `model_catalog_source` | `ModelCatalogSource` | `None` or `ShellCommand { program, args }` |
| `model_env_vars` | `&'static [&'static str]` | Provider-specific MODEL env var chain |
| `cli_sensitive_axes` | `CliSensitiveAxes` | Which permission axes CLI flags can override |
| `repo_home_root_files` | `&'static [&'static str]` | Root files preserved during shadow-HOME isolation |
| `unmapped_native_events` | `&'static [UnmappedNativeEvent]` | Native hook events with no 16-event mapping |

### Linking Facade

One fn-pointer field bridges to the cross-provider linking layer:

- `resource_support_fn`: Returns `&'static ProviderCapabilities`

It is built via `LazyLock` in the provider module and is tested for agreement with the typed catalog. (The former `agent_capabilities_fn` bridge was deleted with the legacy `AgentCapabilities` tree at retirement — see `features/2026-07-02-provider-metadata/design/module-split.md`.)

---

## Phase 4: Mechanical Plumbing Checklist

After generating the provider catalog file, you must wire the new provider into the codebase at several mechanical sites. These are one-line insertions at predictable boundaries.

### Lib Crate

- [ ] **`claudine/lib/src/provider_id.rs`**
  - Add `Provider` enum variant with `repr(usize)` discriminant
  - Increment `PROVIDER_COUNT`
  - Add variant to `PROVIDERS_DISPLAY_ORDER`
  - Add compile-time assertion for discriminant index

- [ ] **`claudine/lib/src/provider/mod.rs`**
  - Add `mod {file};`

- [ ] **`claudine/lib/src/provider/registry.rs`**
  - Import `super::{file}::{PREFIX}_INFO`
  - Add `&{PREFIX}_INFO` to the `REGISTRY` array

- [ ] **`claudine/lib/src/stream/providers/mod.rs`** (if provider has structured streams)
  - Add `pub mod {file};`
  - Add factory arm in `for_provider()`

- [ ] **`claudine/lib/src/adapters/mod.rs`** (if provider has adapter)
  - Add `pub(crate) mod {file};`
  - Export adapter static
  - Add to adapter dispatch match arms

- [ ] **`claudine/lib/src/config/mod.rs`** (if provider has configurator)
  - Add `pub(crate) mod {file};`
  - Export configurator type

### CLI Crate

- [ ] **`claudine/cli/src/main.rs`**
  - Add `Commands::{Provider}(args)` arm mapping to `Provider::{Variant}`
  - Add to the `unreachable!` catch-all match

- [ ] **`claudine/cli/src/commands/wrap/profile/mod.rs`** (if provider needs wrapper overrides)
  - Add `mod {file};`
  - Add `pub(crate) use`
  - Add to `WRAPPER_REGISTRY` static

- [ ] **`claudine/cli/src/commands/wrap/profile/{file}.rs`** (if defaults are insufficient)
  - Create wrapper profile with per-provider overrides

### MCP Crate (if applicable)

- [ ] **`claudine/lib/src/mcp/import.rs`** — Add import parser if provider has custom MCP config format
- [ ] **`claudine/lib/src/mcp/export.rs`** — Add export writer if provider has custom MCP config format

---

## Phase 5: Generate Code

### Recommended Approach: Template + File Edits

Do **not** build a separate code generation library. Instead:

1. **Generate the provider catalog file** (`lib/src/provider/{file}.rs`) from a template populated with research frontmatter data. This is the bulk of the work and is easiest to do with string templating in the prompt.

2. **Use simple text replacement** for the mechanical plumbing sites (enum variants, `mod` declarations, registry entries, CLI mappings). These are well-defined, small, and verifiable.

3. **Let the compiler and invariant tests verify correctness.** The existing test suite catches most gaps:
   - Registry completeness
   - Non-empty mandatory fields
   - Legacy facade agreement
   - Structural invariants
   - Unauthorized `match Provider` in the lib crate (`no_unauthorized_match_provider_in_lib`)

### Why Not a Codegen Library?

- **Frequency**: New providers are added infrequently (a few times per year).
- **Complexity**: A library would need to understand Rust AST semantics to generate valid nested static data and conditional trait implementations safely.
- **Maintenance**: The library itself would need tests and versioning.
- **Philosophy**: Violates Rule 2 (Simplicity First) and Rule 3 (Surgical Changes).

### Why Not Pure Regex?

- The new provider module (~400–600 lines of nested structs) is too complex for regex generation. It needs a template.
- The mechanical changes across files are perfect for regex/file editing.

---

## Phase 6: Verify

### Compile-Time Checks

```bash
cargo check -p claudine
cargo check -p claudine-cli
```

### Test Suite

```bash
# Run provider-specific tests
cargo test -p claudine provider

# Run sniff tests to ensure AiCli consistency
cargo test -p sniff-lib

# Full test suite
cargo test -p claudine -p claudine-cli -p sniff-lib
```

### Key Invariant Tests

| Test | What It Guards |
|------|----------------|
| `registry_completeness` | Every `Provider` variant has a registry entry |
| `non_empty_mandatory_fields` | Identity fields are populated |
| `agent_capabilities_facade_matches_catalog` | Legacy tree agrees with typed catalog |
| `resource_support_facade_matches_catalog` | Legacy tree agrees with typed catalog |
| `config_paths_have_primary_user_entry` | Every provider declares at least one config path |
| `no_unauthorized_match_provider_in_lib` | No new `match Provider` blocks outside allowed files |
| `sniff_ai_cli_maps_all_providers` | Every provider maps to an `AiCli` variant |

### Manual Verification

```bash
# Verify the provider appears in the describe output
cargo run -p claudine-cli -- providers --describe --format json | jq '.[] | select(.slug == "new_provider")'

# Verify detection works
sniff ai-clis
```

---

## Summary: Files Changed Per Provider

| File | Change Type | Effort |
|------|-------------|--------|
| `lib/src/provider/{file}.rs` | New file — full catalog + traits | High |
| `lib/src/provider_id.rs` | Enum variant + count + order | Low |
| `lib/src/provider/mod.rs` | `mod` declaration | Low |
| `lib/src/provider/registry.rs` | Import + registry entry | Low |
| `lib/src/provider/methods.rs` | `sniff_ai_cli` test assertion | Low |
| `lib/src/stream/providers/mod.rs` | Parser module + factory arm (optional) | Medium |
| `lib/src/adapters/{file}.rs` | New file (optional) | Medium |
| `lib/src/adapters/mod.rs` | Module + export + dispatch (optional) | Low |
| `lib/src/config/{file}.rs` | New file (optional) | Medium |
| `lib/src/config/mod.rs` | Module + export (optional) | Low |
| `cli/src/main.rs` | Command mapping | Low |
| `cli/src/commands/wrap/profile/{file}.rs` | New file (optional, if defaults suffice) | Medium |
| `cli/src/commands/wrap/profile/mod.rs` | Module + export + registry (optional) | Low |
| `sniff/lib/src/programs/enums/categories.rs` | `AiCli` variant | Low |
| `sniff/lib/src/programs/enums/metadata.rs` | `ProgramInfo` + install methods + `serde_key` | Low |

**Typical total**: 1 new large file (~500 lines) + ~10–15 one-line mechanical edits across 8–12 files + optional adapter/configurator/wrapper files.

---

## See Also

- [`provider-metadata.md`](./provider-metadata.md) — Deep dive into the `ProviderInfo` catalog design, behavior traits, and known gaps
- [`@claudine/prompts/new-provider.md`](../../prompts/new-provider.md) — The research prompt that drives Phase 1
- `sniff` library — Cross-platform program detection and installation planning
