# Linking Strategy

## Contents

- Why Linking Exists
- Resource Types
- Scopes
- Algorithm Phases
- Key Types
- Provider Skill Paths
- Provider Command Paths
- Conflict Detection and Resolution

Use heading search to jump to the listed subsystem.


Claudine's linking system synchronizes skills, commands, agents, and scripts across the compiled agentic CLI provider catalog. The goal is **write once, use everywhere**: author a resource in one canonical provider and have it appear in all compatible providers via symlinks or format-converted derived artifacts.

## Why Linking Exists

Each agentic CLI reads resources from its own directory tree. Without linking, a user who writes a skill for one provider must manually copy or re-create it for every compatible provider. Claudine automates this by:

1. Electing a **canonical provider** per resource type and scope
2. **Discovering** resources across all provider directories
3. **Classifying** each resource's sync state (missing, linked, stale, conflicting)
4. **Applying** symlinks for same-format providers and format-converted derived files for different-format providers

## Resource Types

Four kinds of resources can be linked:

| Resource | Discovery Pattern | Hash Strategy |
|----------|-------------------|---------------|
| **Skill** | Directory with `SKILL.md` entry point | Recursive walk, sorted by relative path, null-separated, xxHash |
| **Command** | Single `.md` file (slash commands) | File content xxHash |
| **Agent** | Single `.md`/`.yaml`/`.toml` file | File content xxHash |
| **Script** | Executable file | File content xxHash |

## Scopes

Linking operates in two independent scopes:

| Scope | Source Paths | Symlink Style |
|-------|-------------|---------------|
| **User** | `~/.{provider}/skills/`, `~/.config/{provider}/skills/`, etc. | Absolute symlinks |
| **Repo** | `{repo}/.{provider}/skills/`, etc. | Relative symlinks (portable across machines) |

## Algorithm Phases

### Phase 1: Canonical Provider Selection

A canonical provider is elected per `(scope, resource_type)` pair. The selection algorithm (`canonical.rs`):

1. Build a preference ordering from user-configured preferences, then append remaining installed providers alphabetically
2. Filter candidates to only those with **Markdown format** and **custom support level**
3. Exclude providers whose entrypoint directory is itself a symlink (already a link consumer)
4. Prefer the first candidate with **valid assets** (existing non-hidden skill directories with `SKILL.md`, or `.md` command files)
5. Fall back to the first candidate even without assets

The canonical provider setting is persisted per slot in `CanonicalProviderSettings` (8 slots: 2 scopes x 4 resource types).

### Phase 2: Discovery

Two discovery systems operate in parallel:

**Legacy discovery** (`discovery.rs`) -- used by the `link_skills()` entry point:
- Scans each provider's skill/command directories for the given scope
- Skills: directories containing `SKILL.md`, skipping hidden directories
- Commands: `.md` files, skipping hidden files
- Records `is_symlink` flag via `symlink_metadata()`
- Sorts results by `(name, provider_slug)` for deterministic processing

**Detector-based discovery** (`detector.rs`) -- used by the `analyze_resource_links()` entry point:
- Per-resource-type detector structs: `SkillsDetector`, `SlashCommandsDetector`, `AgentDefinitionsDetector`, `SharedScriptsDetector`
- All implement the `LinkDetector` trait via a shared macro
- Separate user/repo scope state with canonical resource maps
- Format-aware: reads `.md`, `.toml`, `.yaml`, or executable files depending on provider capability metadata

### Phase 3: Hashing

- **Skill directories**: `hash_skill_dir()` walks all files recursively (following symlinks), sorts by relative path, concatenates `relative_path + NUL + content + NUL` for each file, then computes `biscuit_hash::xx_hash_bytes()`
- **Single files** (commands, agents, scripts): direct `biscuit_hash::xx_hash_bytes()` on file content
- **Frontmatter hash** (compatibility module): sorted `key + NUL + value + NUL` pairs, xxHash
- **Body hash**: xxHash on the markdown body content (after frontmatter delimiter)

Symlinks are resolved before hashing so a symlink and a real directory with identical content produce the same hash.

### Phase 4: Conflict Analysis

The analysis (`conflict.rs`) groups discovered resources by name and classifies each group into one of four states:

| Status | Condition | Action |
|--------|-----------|--------|
| `LinkCandidate` | Resource exists in exactly one provider (non-symlink) | Create symlinks to other providers |
| `InSync` | Multiple providers have identical content hashes | No action needed |
| `Conflict` | Multiple providers have different content hashes | Report to user, no automatic resolution |
| `AlreadyLinked` | One non-symlink source + symlink copies exist | No action needed |

**Also-reads-from filtering**: When a target provider already reads from the source provider's directory (e.g., OpenCode reads `.claude/skills/`), it is excluded from `LinkCandidate.target_providers` to avoid redundant symlinks.

### Phase 5: Compatibility Classification

The compatibility module (`compatibility.rs`) performs deeper analysis for the detector-based workflow:

1. **Canonical candidate classification**: Parses frontmatter, applies deterministic upgrades (alias duplication, name derivation), then classifies as `Source` or `PartialSource` based on missing required properties across all providers
2. **Target reference classification**: For each non-canonical provider, checks whether required properties are satisfied to determine `LinkMissing` vs `IncompleteLink`
3. **Alias duplication**: When providers use different property names for the same concept (e.g., `max_turns` vs `maxTurns`), both are written to the canonical source
4. **Name derivation**: If `name` is missing from frontmatter, infers it from the directory name (skills) or file stem (commands/agents) and writes it in place

### Phase 6: Symlink / Derived Artifact Creation

**Direct links** (same format -- `symlink.rs`):
- User scope: absolute symlink from source to `dest_dir/{name}`
- Repo scope: relative symlink computed via `relative_path()` (counts `..` components from common prefix)
- Never overwrites real (non-symlink) directories
- Detects existing symlinks: reports `AlreadyLinked` if pointing correctly, `Skipped` if pointing elsewhere

**Derived artifacts** (different format -- `execution.rs`):
- Markdown to TOML: Converts frontmatter keys to TOML values, body to `prompt` key, appends `_claudine_fm_hash` and `_claudine_body_hash` for staleness detection
- Markdown to YAML: Same conversion via `serde_yaml`
- Bespoke converters: Function pair `(to_canonical, from_canonical)` for custom format bridges
- Staleness: Detected by comparing embedded hash values against canonical source hashes

**Category-level symlinks**: If a provider's entire resource root directory (e.g., `.opencode/skills/`) is itself a symlink, Claudine reports it as skipped and does not create individual symlinks underneath it.

## Key Types

### `LinkableResource` (capabilities.rs)

```
Skill     -- Directory bundles with SKILL.md entry point
Command   -- Slash commands (custom prompts invoked via /name)
Agent     -- Subagent/persona definitions for task delegation
Script    -- Executable scripts invoked by skills/agents
```

### `ResourceFormat` (capabilities.rs)

```
Markdown    -- Most common, used by Claude, Codex, Gemini (skills), OpenCode, Qwen
Toml        -- Gemini commands
Yaml        -- Goose recipes, KimiCode agents
Mcp         -- Goose commands (not file-based)
BuiltinOnly -- KimiCode commands (no custom file support)
Executable  -- Scripts (Codex, Goose, OpenCode, Qwen)
```

### `SupportLevel` (capabilities.rs)

```
Full         -- Full support with custom file creation (symlinkable)
CustomFormat -- Supported but uses different format (requires derived artifact)
Limited      -- Built-in only, no custom resources
None         -- Not supported by this provider
```

### `ResourceReference` (model.rs)

The detailed state of a resource at a specific provider:

```
Source(definition)                   -- Canonical source with all required properties
PartialSource(definition, missing)   -- Canonical source missing some required properties
Isolated(definition)                 -- Standalone resource not linked from canonical
Link(provider, scope)                -- Symlink to canonical source (healthy)
LinkMissing(provider, scope)         -- Symlink should exist but is missing (fixable)
IncompleteLink(provider, scope)      -- Cannot link due to incomplete canonical or unsatisfied requirements
DerivedLink(provider, scope)         -- Derived representation exists and matches hashes (healthy)
DerivedStale(provider, scope)        -- Derived representation exists but hashes differ (fixable)
DerivedMissing(provider, scope)      -- Derived representation should exist but is missing (fixable)
```

Each variant maps to a `ReferenceStatus`: `Ok`, `IsFixable`, or `NeedsUserAttention`.

### `SkillSyncStatus` (conflict.rs)

Legacy status classification from `analyze_skills()`:

```
LinkCandidate { source, target_providers }  -- Single source, ready to link
InSync { name, providers }                  -- All copies match
Conflict { name, versions }                 -- Copies differ
AlreadyLinked { name, source_provider }     -- Already symlinked
```

### `ProviderPaths` / `ProviderSkillPaths` (paths.rs)

Path resolution for all providers. `ProviderPaths` holds per-provider directories (user/repo for skills/commands) plus `also_reads_from` lists. `ProviderSkillPaths` is derived from the complete compiled catalog with resolved home and repo root paths.

### `ProviderCapabilities` (capabilities.rs)

Complete capability metadata per provider: skill/command/agent/script support details, frontmatter schema (required/optional properties), format, paths, and also-reads-from relationships.

## Provider Skill Paths

| Provider | User Skills | Repo Skills | Also Reads From |
|----------|-------------|-------------|-----------------|
| Claude | `~/.claude/skills` | `.claude/skills` | -- |
| Codex | `~/.codex/skills` | `.codex/skills` | `.claude/skills`, `.agents/skills` |
| Gemini | `~/.gemini/skills` | `.gemini/skills` | -- |
| Goose | `~/.config/goose/skills` | `.goose/skills` | `.claude/skills`, `.agents/skills` |
| KimiCode | `~/.config/agents/skills` | `.kimi/skills` | `.claude/skills`, `.agents/skills`, `.codex/skills` |
| OpenCode | `~/.config/opencode/skills` | `.opencode/skills` | `.claude/skills`, `.agents/skills` |
| QwenCode | `~/.qwen/skills` | `.qwen/skills` | -- |
| Kilo | `~/.config/kilo/skills` | `.kilo/skills` | `.claude/skills`, `.agents/skills` |
| Pi | `~/.pi/agent/skills` | `.pi/skills` | `.agents/skills` |
| Antigravity | `~/.gemini/config/skills` | `.agents/skills` | -- |

## Provider Command Paths

| Provider | User Commands | Repo Commands | Format |
|----------|-------------- |---------------|--------|
| Claude | `~/.claude/commands` | `.claude/commands` | Markdown |
| Codex | `~/.codex/prompts` | `.codex/prompts` | Markdown (deprecated) |
| Gemini | `~/.gemini/commands` | `.gemini/commands` | TOML (derived) |
| Goose | -- | -- | MCP (not file-based) |
| KimiCode | -- | -- | Built-in only |
| OpenCode | `~/.config/opencode/commands` | `.opencode/commands` | Markdown |
| QwenCode | `~/.qwen/commands` | `.qwen/commands` | Markdown |
| Kilo | `~/.config/kilo/commands` | `.kilo/commands` | Markdown |
| Pi | `~/.pi/agent/prompts` | `.pi/prompts` | Markdown |
| Antigravity | -- | -- | Skill-derived only |

## Conflict Detection and Resolution

**Hash-based conflict detection**: When the same resource name exists in multiple providers as non-symlink directories/files, their content hashes are compared. Matching hashes mean `InSync`; differing hashes mean `Conflict`.

**Automatic resolution (fixable states)**:
- `LinkMissing`: Create a symlink from canonical source to target provider directory
- `DerivedMissing`: Generate a format-converted artifact with embedded hash markers
- `DerivedStale`: Regenerate the derived artifact with updated content and hashes

**Manual resolution required**:
- `Conflict`: Different content in multiple providers -- user must choose which version to keep
- `PartialSource`: Canonical source missing required properties for some providers
- `IncompleteLink`: Target provider's requirements cannot be satisfied from the canonical source

**Safety invariants**:
- Never overwrites real (non-symlink) directories or files
- Category-level symlinks (entire resource root is a symlink) are detected and preserved
- Scope isolation: User-scope sources are not linked into repo-scope targets and vice versa
