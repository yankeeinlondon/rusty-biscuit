# Model Property Design

The `model` property on agent and command definitions presents a unique cross-linking challenge. Unlike most frontmatter properties -- which are either universally applicable or safely ignorable -- a model ID is inherently provider-specific. A Claude Code agent definition specifying `model: claude-sonnet-4-5` cannot be symlinked to Codex and have that model value work. This document designs claudine's approach to handling model properties across agentic CLIs, progressing from a simple flagging mechanism to full capability-based model translation.

## Problem Statement

Both agents and commands can specify a `model` property, but each agentic CLI only supports models from specific providers:

| Agentic CLI   | Model Ecosystem                           | model on Skills | model on Commands | model on Agents |
|---------------|-------------------------------------------|:---------------:|:-----------------:|:---------------:|
| Claude Code   | Anthropic only                            | yes             | yes               | yes             |
| Codex         | OpenAI only                               | no              | no                | no              |
| Gemini CLI    | Google only                               | no              | no                | yes             |
| OpenCode      | Multi-provider (any OpenAI-compatible API) | no           | yes               | yes             |
| Goose         | Multi-provider (configured per-profile)   | no              | no                | no              |
| KimiCode      | Moonshot/Kimi only (6 k2 variants)        | no              | no                | no              |
| Qwen Code     | Qwen only (coder-model, vision-model)     | no              | no                | no              |
| Roo Code      | Multi-provider (VS Code extension config) | no              | no                | no              |

Key observations:

1. Only **Claude Code**, **Gemini CLI**, and **OpenCode** support the `model` property on agents/commands at all. Claude Code additionally supports `model` on **skills** (the only provider to do so).
2. Of those three, Claude and Gemini are single-provider ecosystems while OpenCode is multi-provider.
3. CLIs that lack a `model` property simply use whatever model is configured globally -- the property is silently ignored or causes an error.
4. A symlinked resource with a hardcoded model ID will either fail, be ignored, or use an inaccessible model on the target CLI.


## Phase 1: Flag and Report

**Status**: Specified in `claudine/docs/cli/commands.md` and `claudine/docs/cli/agents.md`.

The simplest approach: any command or agent definition that contains a `model` property is flagged as **not shareable** in the Exceptions area of `claudine commands` and `claudine agents` reports.

### Behavior

- During resource discovery, if a command or agent definition has a `model` key in its frontmatter, record this fact.
- In report output, list these resources under the Exceptions badge with the reason: "contains model property (provider-specific)".
- The resource is still linked to other CLIs if all other compatibility checks pass, but the `model` property is called out as a portability concern.
- No automatic transformation occurs.

### Limitations

- Resources with `model` properties are effectively second-class citizens in the linking system.
- Users who want cross-CLI portability must manually remove or adapt model properties.
- No path exists for automated model translation.


## Phase 2: Capability-Based Model Translation

Phase 2 introduces automatic model property translation using `unchained-ai`'s `ModelCapability` abstraction as the bridge between provider-specific model IDs and portable capability descriptions.

### Architecture Overview

```
                     unchained-ai                          claudine
                ┌─────────────────────┐           ┌──────────────────────┐
                │                     │           │                      │
                │  ProviderModel      │           │  CliModelMapping     │
                │    (model catalog)  │◄──────────│    (CLI defaults)    │
                │                     │           │                      │
                │  ModelCapability    │           │  UserModelPrefs      │
                │    (capability      │◄──────────│    (overrides from   │
                │     abstraction)    │           │     claudine.toml)   │
                │                     │           │                      │
                │  model_id ──►       │           │  ModelTranslator     │
                │    ModelCapability   │           │    (conversion       │
                │                     │           │     engine)          │
                └─────────────────────┘           └──────────────────────┘
```

**Ownership split**:

- **unchained-ai** owns the model catalog (`ProviderModel`), the capability abstraction (`ModelCapability`), and the mapping from specific model IDs to capabilities. This is reusable knowledge about models that is not claudine-specific.
- **claudine** owns the mapping from `ModelCapability` to the concrete model ID each agentic CLI should use, user preference overrides, and the translation engine that rewrites model properties during link/fix operations.

### Capability Tier Selection

Not all `ModelCapability` variants are relevant for agent/command model translation. The full `ModelCapability` enum includes fine-grained variants for thinking modes, temperature presets, and cost optimizations that agentic CLIs do not control via the `model` property. For model translation, we use a **simplified tier** derived from the primary capability axis:

| Simplified Tier | Maps from ModelCapability variants                                      | Intent                                       |
|-----------------|-------------------------------------------------------------------------|----------------------------------------------|
| `Fast`          | `FastCheap`, `Fast`                                                     | Quick responses, simple tasks                |
| `Normal`        | `Normal`, `NormalCheap`, `NormalThinking`, `NormalThinkingCheap`, `NormalUltrathink`, `NormalCheapUltrathink` | General-purpose coding tasks    |
| `Smart`         | `Smart`, `SmartCheap`, `SmartThink`, `SmartCheapThink`, `SmartUltrathink`, `SmartCheapUltrathink`           | Complex reasoning, architecture |

The `Creative*` and `Literal*` variants map to their corresponding tier (`CreativeFast` -> `Fast`, `CreativeNormal` -> `Normal`, `CreativeSmart` -> `Smart`, and likewise `LiteralFast` -> `Fast`, `LiteralNormal` -> `Normal`, `LiteralSmart` -> `Smart`). Temperature modifiers are handled separately via the `temperature` property, which is already portable across CLIs that support it (Gemini agents and OpenCode agents both accept `temperature`; OpenCode agents also accept `top_p`).

The `Specific(ProviderModel)` variant in `ModelCapability` is a special case -- it pins to an exact model and provider, which is inherently non-portable and falls back to the Phase 1 "flag as exception" behavior unless the specific model happens to be available on the target CLI. Note that each provider-specific model enum also has a `Bespoke(String)` variant for unrecognized model IDs, which would similarly be untranslatable.

### Model ID to Capability Mapping

This mapping lives in **unchained-ai** because it is general knowledge about models. A new function on `ProviderModel` (or a standalone lookup) maps known model IDs to their simplified tier:

```
claude-sonnet-4-5           -> Normal     (ProviderModelAnthropic::Claude__Sonnet__4__5__20250929)
claude-opus-4-5             -> Smart      (ProviderModelAnthropic::Claude__Opus__4__5__20251101)
claude-haiku-4-5            -> Fast       (ProviderModelAnthropic::Claude__Haiku__4__5__20251001)
o3                          -> Smart      (ProviderModelOpenAi::O3)
o4-mini                     -> Normal     (ProviderModelOpenAi::O4__Mini)
gpt-5.1-codex              -> Normal     (ProviderModelOpenAi::Gpt__5_1__Codex)
gemini-2.5-pro              -> Smart      (ProviderModelGemini::Gemini__2_5__Pro)
gemini-2.5-flash            -> Normal     (ProviderModelGemini::Gemini__2_5__Flash)
gemini-2.0-flash-lite       -> Fast       (ProviderModelGemini::Gemini__2_0__Flash__Lite)
kimi-k2-thinking            -> Smart      (ProviderModelMoonshotAi::Kimi__K2__Thinking)
deepseek-chat               -> Normal     (ProviderModelDeepseek::Deepseek__Chat)
```

Note: `qwen-coder-plus` is not currently in the unchained-ai model catalog and would need to be added or handled as a `Bespoke` variant.

When a model ID is not recognized (e.g., a `Specific(ProviderModel)` variant wrapping a `Bespoke` model string), the mapping returns `None` and the model property is treated as a Phase 1 exception (flagged, not translated).

### CLI Default Model Table

The per-CLI default model table lives in **claudine** because it encodes opinions about which specific model each agentic CLI should use at each capability tier. This is distinct from "what models exist" (unchained-ai's domain) -- it is "what model should Codex use when the intent is Smart-tier work."

The table structure maps `(Provider, SimplifiedTier)` -> `Option<model_id>`:

| Agentic CLI   | Fast                    | Normal                  | Smart                    |
|---------------|-------------------------|-------------------------|--------------------------|
| Claude Code   | `claude-haiku-4-5`      | `claude-sonnet-4-5`     | `claude-opus-4-5`        |
| Codex         | --                      | `o4-mini`               | `o3`                     |
| Gemini CLI    | `gemini-2.0-flash-lite` | `gemini-2.5-flash`      | `gemini-2.5-pro`         |
| OpenCode      | *(user-configured)*     | *(user-configured)*     | *(user-configured)*      |
| Goose         | *(n/a)*                 | *(n/a)*                 | *(n/a)*                  |
| KimiCode      | *(n/a)*                 | *(n/a)*                 | *(n/a)*                  |
| Qwen Code     | *(n/a)*                 | *(n/a)*                 | *(n/a)*                  |
| Roo Code      | *(n/a)*                 | *(n/a)*                 | *(n/a)*                  |

Notes:
- Entries marked `--` mean the CLI supports the model property but has no model at that tier (Codex has no "fast" model in the traditional sense).
- Entries marked *(n/a)* mean the CLI does not support the `model` property on agents/commands at all. The model property is dropped during conversion.
- Entries marked *(user-configured)* mean the CLI supports arbitrary providers and the default must come from user configuration.

### User Configuration

Users can override the default model table via `claudine.toml`. This handles two scenarios:

1. **Preference overrides**: A user considers `claude-sonnet-4-5` their "Smart" model (they don't use Opus).
2. **Multi-provider CLIs**: OpenCode and Goose (if it gains model property support) support arbitrary providers, so they need user input to know which models are *preferred* at each tier.

Configuration structure in `claudine.toml`:

```toml
[model_mapping]
# Override default tier assignments for specific CLIs

[model_mapping.claude]
fast = "claude-haiku-4-5"
normal = "claude-sonnet-4-5"
smart = "claude-sonnet-4-5"   # user prefers Sonnet for "smart" tasks too

[model_mapping.opencode]
fast = "deepseek-chat"
normal = "anthropic/claude-sonnet-4-5"
smart = "anthropic/claude-opus-4-5"
```

When a mapping is needed and no user override exists, the built-in default table is used. When neither a default nor an override exists for a target CLI, the model property is dropped (the CLI will use its globally configured model).

### User Interview Flow

During `claudine init`, if the user has CLIs installed that support multi-provider model configuration (OpenCode, Goose), prompt for their model preferences:

```
OpenCode supports multiple model providers.
Which models do you use with OpenCode?

  Fast tier (quick tasks):    [deepseek-chat]
  Normal tier (general):      [anthropic/claude-sonnet-4-5]
  Smart tier (complex):       [anthropic/claude-opus-4-5]
```

This interview is:
- **Skipped** if `[model_mapping.opencode]` already exists in `claudine.toml`.
- **Optional** -- the user can press Enter to skip any tier, meaning that tier will have no mapping and the model property will be dropped for that CLI.
- **Re-triggered** if the user runs `claudine init` again and new multi-provider CLIs are detected.

For single-provider CLIs (Claude, Gemini), the defaults are known and no interview is needed unless the user wants to override tier assignments.

### Translation During Link/Fix Operations

When claudine creates or updates a derived resource (not a symlink -- derived resources are generated copies), the model property is translated:

1. **Read** the canonical source's `model` value (e.g., `claude-sonnet-4-5`).
2. **Resolve** the model ID to a simplified capability tier via unchained-ai (e.g., `Normal`).
3. **Look up** the target CLI's model for that tier (checking user overrides first, then defaults).
4. **Write** the translated model ID into the derived resource, or **drop** the model property if no mapping exists.

For **symlinked** resources (where source and target share the same file), model translation is not possible. This means:
- If the canonical source has a model property and the target CLI uses symlinks, the resource is flagged as a Phase 1 exception.
- Model translation only applies to **derived** resources (generated copies in a different format or with transformed frontmatter).

This has an architectural implication: CLIs that currently use symlinks for agent definitions may need to switch to derived copies if model translation is desired. The `ResourceFormatConversion` enum in `linking/model.rs` already supports this via the `Bespoke` variant.

### Handling "model" on Symlinked Resources

When a resource is symlinked (not derived), the model property creates an inherent conflict. Options:

1. **Accept the conflict**: Symlink the resource anyway, flag in the report. The target CLI may ignore the unknown model, use its global default, or error. This is the Phase 1 behavior.
2. **Promote to derived**: Instead of symlinking, generate a derived copy with the model property translated. This gives correctness at the cost of losing automatic content synchronization.
3. **Strip on link**: Create a derived copy that is identical to the source except with the model property removed. This is a middle ground -- the resource works everywhere but loses the model intent.

The recommended approach is **option 2** (promote to derived) when:
- The canonical source has a model property.
- The target CLI supports the model property.
- A valid mapping exists for the target CLI.

And **option 3** (strip on link) when:
- The target CLI does not support the model property at all.

And **option 1** (accept/flag) when:
- No mapping can be determined (unrecognized model ID, no user config).


## Data Model

### New types in claudine

```
/// Simplified capability tier for model translation.
/// Derived from unchained-ai's ModelCapability but collapsed
/// to the three axes relevant for agent/command model properties.
enum ModelTier {
    Fast,
    Normal,
    Smart,
}
```

```
/// Per-CLI model mapping entry.
/// Maps a ModelTier to the concrete model ID string that CLI should use.
struct CliModelDefaults {
    cli: Provider,
    fast: Option<String>,
    normal: Option<String>,
    smart: Option<String>,
}
```

```
/// User-configured model mapping overrides from claudine.toml.
/// Keys are Provider variants, values override CliModelDefaults.
struct UserModelMapping {
    overrides: HashMap<Provider, CliModelDefaults>,
}
```

```
/// Result of attempting to translate a model property.
enum ModelTranslation {
    /// Successfully translated to target CLI's model ID.
    Translated { source: String, target: String, tier: ModelTier },
    /// Model property should be dropped (target CLI doesn't support it).
    Drop { source: String, reason: String },
    /// Cannot translate -- flag as exception.
    Untranslatable { source: String, reason: String },
}
```

### New function in unchained-ai

```
/// Map a ProviderModel to its simplified capability tier.
/// Returns None for Bespoke variants or models without known classification.
fn model_tier(model: &ProviderModel) -> Option<ModelTier>
```

This function could alternatively be implemented as a method on `ProviderModel` or derived from `ModelMetadata` if the metadata includes a tier/class field.


## Integration Points

### claudine linking pipeline

The model translation integrates into the existing linking pipeline at the point where derived resources are generated:

1. **Discovery** (`linking/discovery.rs`): No change. Model properties are discovered as part of frontmatter parsing.
2. **Compatibility** (`linking/compatibility.rs`): Extended to check whether a model property creates a compatibility issue. Uses `ModelTranslation` to determine if the resource should be promoted from symlink to derived.
3. **Execution** (`linking/execution.rs`): Extended to apply model translation when generating derived resources. The `ResourceFormatConversion::Bespoke` variant gains a model-aware converter.
4. **Report** (`linking/report.rs`): Extended to show model translation details in the Exceptions section (Phase 1 behavior) and in the resource detail view (Phase 2 translations).

### claudine CLI commands

- `claudine commands`: Report shows model property status per command.
- `claudine agents`: Report shows model property status per agent.
- `claudine fix`: Applies model translations when generating derived resources.
- `claudine init`: Interviews user for multi-provider CLI model preferences.

### unchained-ai dependency

claudine already depends on workspace-internal crates. Adding a dependency on `unchained-ai` for `ModelCapability` and `ProviderModel` is straightforward. The dependency is for type definitions and lookup functions only -- no runtime services.

**Important type disambiguation**: The `Provider` enum in unchained-ai (`unchained_ai::rigging::providers::Provider`) represents LLM API providers (Anthropic, OpenAI, Gemini, DeepSeek, etc.), while claudine's `Provider` enum (`claudine::events::Provider`) represents agentic CLI products (Claude Code, Codex, Gemini CLI, OpenCode, etc.). These are different taxonomies -- e.g., OpenCode (claudine Provider) can use models from Anthropic, OpenAI, or DeepSeek (unchained-ai Providers). The `CliModelDefaults` struct maps claudine's `Provider` to model ID strings, not unchained-ai's `Provider`. The model ID to tier lookup goes through unchained-ai's `ProviderModel`, which internally knows its unchained-ai `Provider`.

### capabilities.rs alignment

The `ResourcePropertySchema` constants in `linking/capabilities.rs` already record which CLIs support the `model` property. The model translation system reads this to determine whether a target CLI can accept a translated model value or whether the property should be dropped.


## Edge Cases and Limitations

### Unrecognized model IDs

If a model ID is not in unchained-ai's catalog (e.g., a very new model, a fine-tuned model, or a typo), it cannot be mapped to a tier. These fall back to Phase 1 exception behavior.

### Model availability drift

The default model table will become stale as providers release new models and deprecate old ones. Mitigation:
- Keep defaults in a single location (`CliModelDefaults`) that is easy to update.
- unchained-ai's model catalog is already auto-generated from provider APIs, so new models are picked up during regeneration.
- User overrides in `claudine.toml` take precedence, so users are never blocked by stale defaults.

### Multi-provider CLIs with unknown configuration

OpenCode supports any OpenAI-compatible API, meaning it can access models from virtually any provider. Goose is similarly broad. Unlike single-provider CLIs where the default model table can be hardcoded, these CLIs need user input to know which models are *preferred* at each tier. Options:
- Ask the user during `claudine init` (the chosen approach).
- Accept that without user input, model translation is unavailable for these CLIs.

### Temperature and top_p interaction

Some `ModelCapability` variants (Creative, Literal) imply temperature modifications. When translating a model property, if the source capability variant implies a temperature change and the target CLI supports a `temperature` property, claudine could add/adjust it. However, this is out of scope for the initial implementation -- temperature/top_p are already portable and should be handled orthogonally.

### Skills with model properties

Claude Code is the only provider that supports `model` on skills (via the `CLAUDE_SKILL_SCHEMA`). When a skill is linked to other providers, the model property must be stripped since no other provider recognizes it. This is simpler than the agent/command case -- there is no translation needed, only removal. However, skills are currently always symlinked (not derived), so this is another case where symlink-to-derived promotion would be necessary to strip the property.

### IncompleteCause extension

Phase 2 will require extending the `IncompleteCause` enum in `linking/model.rs` with a new variant to represent model-related incompatibilities:

```rust
/// Model property cannot be translated for the target provider.
UntranslatableModel { model_id: String, reason: String },
```

This integrates with the existing `ResourceReference::IncompleteLink` variant and the `ReferenceStatus::NeedsUserAttention` status classification.

### Roo Code modes

Roo Code agents (modes) do not have a `model` property in their schema, but Roo Code's VS Code extension allows per-mode model configuration through the UI. This configuration is not accessible via file-based resource definitions, so model translation does not apply to Roo Code.


## Open Questions

1. **Should ModelTier live in unchained-ai or claudine?** It is a simplification of `ModelCapability` and could be useful outside claudine. On the other hand, it is an opinionated reduction that may not suit all consumers. Current recommendation: define in claudine, with a `From<ModelCapability>` conversion that references unchained-ai types.

2. **Should the default model table be code or config?** Hardcoding in Rust makes it type-safe and version-controlled but requires recompilation to update. Externalizing to a TOML/YAML file makes it user-editable but adds a file discovery concern. Current recommendation: hardcode defaults in Rust, allow user overrides via `claudine.toml`.

3. **When should symlink-to-derived promotion happen?** Automatically during `claudine fix`, or only when explicitly requested? Automatic promotion changes the linking topology which may surprise users. Current recommendation: automatic during `claudine fix` with a report line explaining the promotion reason.

4. **Should claudine parse OpenCode/Goose config files?** OpenCode supports any OpenAI-compatible API (config is in `opencode.jsonc`, not `opencode.json` as previously assumed), so parsing its config would only reveal which models are *registered*, not which are *available*. Goose is similarly broad. Current recommendation: rely on user configuration via `claudine.toml`, since the question is about tier *preferences* rather than provider *availability*.

5. **How should version-pinned model IDs be handled?** Model IDs like `claude-opus-4-5-20251101` include a version date. Should translation preserve the pin or map to the latest? Current recommendation: map pinned IDs to the same tier as their unpinned equivalent, and use the unpinned form in the target CLI (e.g., `claude-opus-4-5-20251101` -> Smart -> `gemini-2.5-pro`, not `gemini-2.5-pro-preview-09-2025`). Note: the unchained-ai catalog currently only includes pinned (dated) variants (e.g., `Claude__Opus__4__5__20251101`), not unpinned aliases like `claude-opus-4-5`. The tier lookup should either add unpinned aliases to the catalog or strip date suffixes during lookup to support both forms. Users typically write unpinned IDs in their agent definitions (e.g., `model: claude-opus-4-5`), which would hit the `Bespoke` fallback unless handled.

6. **Should there be a "passthrough" mode for multi-provider CLIs?** OpenCode could potentially accept any `provider/model-id` format. If the source model ID happens to work on the target CLI without translation, should claudine pass it through? Current recommendation: yes, if the target CLI is multi-provider and the source model's provider is one of its configured providers, pass the model ID through unchanged.


## Review Notes

*Review performed 2026-03-04 against codebase at commit f9eb266.*

### Changes made

1. **Fixed model IDs throughout**: Replaced speculative future model IDs (`claude-sonnet-4-6`, `claude-opus-4-6`) with actual current IDs from the unchained-ai catalog (`claude-sonnet-4-5`, `claude-opus-4-5`). The catalog (generated 2026-01-11) shows the latest Anthropic models as `claude-sonnet-4-5-20250929`, `claude-opus-4-5-20251101`, and `claude-haiku-4-5-20251001`.

2. **Added skills column to Problem Statement table**: Claude Code supports `model` on skills (`CLAUDE_SKILL_SCHEMA` in `capabilities.rs` lists `"model"` as optional). No other provider supports `model` on skills. Added observation noting this.

3. **Added ProviderModel variant references**: The Model ID to Capability Mapping table now includes the actual enum variant names from the auto-generated provider model files for traceability.

4. **Removed `qwen-coder-plus` from mapping table**: This model ID is not present in the unchained-ai catalog. Added a note that it would need to be added or handled as a `Bespoke` variant.

5. **Clarified `Specific` vs `Bespoke` terminology**: The `ModelCapability` enum uses `Specific(ProviderModel)` for exact model pinning, while each provider-specific enum uses `Bespoke(String)` for unrecognized model IDs. These are different concepts at different levels.

6. **Added Skills with model properties section**: Documented that Claude Code is the only provider supporting `model` on skills, and that linking skills to other providers requires stripping the property (which implies symlink-to-derived promotion).

7. **Added IncompleteCause extension section**: Phase 2 will need a new `UntranslatableModel` variant in the `IncompleteCause` enum to integrate with the existing `ResourceReference::IncompleteLink` pattern.

8. **Added Provider type disambiguation**: Clarified that unchained-ai's `Provider` (LLM API providers) and claudine's `Provider` (agentic CLI products) are different types serving different taxonomies. This is critical for understanding the `CliModelDefaults` struct.

9. **Expanded Literal variant tier mappings**: Made the `Literal*` -> tier mappings explicit alongside the `Creative*` mappings.

10. **Added temperature/top_p provider support details**: Noted which specific providers support `temperature` (Gemini agents, OpenCode agents) and `top_p` (OpenCode agents) based on `ResourcePropertySchema` constants in `capabilities.rs`.

11. **Corrected version-pinned model ID guidance**: The catalog only has pinned (dated) variants, not unpinned aliases. Added note that the tier lookup needs to handle unpinned IDs (which users are likely to write) either by adding aliases to the catalog or by stripping date suffixes.

### Remaining concerns

- **Codex agent model support**: The `codex_capabilities()` function creates agent support as `ResourceSupport::full()` but the agent schema has no `ResourcePropertySchema` attached (no `.with_properties()` call). This means Codex agent properties are not validated. If Codex adds `model` support to agents in the future, the schema will need updating.
- **Model catalog staleness**: The auto-generated model catalog is from 2026-01-11. Running `gen-models` would refresh it with any new models released since then.
- **No `model_tier()` function exists yet**: The design proposes `fn model_tier(model: &ProviderModel) -> Option<ModelTier>` but this function does not exist in unchained-ai. Implementation should decide whether this belongs as a method on `ProviderModel` or as a standalone function, and whether the mapping is hardcoded or derived from `ModelMetadata`.
- **Unpinned model ID aliases missing from catalog**: The design uses unpinned IDs like `claude-opus-4-5` in the default model table, but the unchained-ai catalog only contains pinned variants (`claude-opus-4-5-20251101`). Either the catalog needs unpinned alias support, or the tier lookup needs fuzzy matching. This is a prerequisite for Phase 2 working correctly with real-world agent definitions.
