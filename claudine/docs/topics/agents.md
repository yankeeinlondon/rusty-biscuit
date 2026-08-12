# Agent Reporting with Claudine CLI

The `claudine agents [FILTER...]` subcommand reports on the current state of **agent/subagent definitions** linking from both a **User** and **Repo** based perspective (if CWD is not a git repo then only User scoped).

- **IMPORTANT:** This file describes the functional and reporting requirements for a CLI command but we need to make sure we are always focused on the division of responsibilities between a CLI and its underlying library: a CLI is for reporting only, all business logic MUST go into the library!

## CLI Arguments

| Argument / Flag     | Description                                                                 |
|---------------------|-----------------------------------------------------------------------------|
| `FILTER`            | Positional, repeatable. Filter agents by name. Supports negation (`-goose` or `!goose`) and exact match (`goose!`). |
| `--apply` / `--fix` | Fix missing agent links for non-canonical providers.                        |
| `-v` / `--verbose`  | Global flag. Forces verbose rendering when more than 1 agent matches.       |

## Key Differences from Skills

Agents differ from skills in several important ways that affect discovery, validation, and linking:

1. **Single files, not directory bundles.** Most providers define agents as individual files (e.g., `researcher.md`) rather than skill-style directories containing `SKILL.md` plus supporting files.
2. **Format heterogeneity.** While skills are universally Markdown with YAML frontmatter, agents span Markdown (Claude, Codex, Gemini, OpenCode, Qwen), YAML (Goose recipes, KimiCode agents) — each with a different schema.
3. **Terminology divergence.** Goose calls them "recipes", KimiCode uses "agents" but with a YAML-only schema. The CLI normalizes all of these under the "agents" umbrella.
4. **No `FileSystem` tree.** Since agents are single files, the Detail View shows file content (frontmatter and opening lines) rather than a directory tree.
5. **Canonical provider must use Markdown.** Because cross-format conversion (Markdown ↔ YAML) is out of scope, only Markdown-based providers are eligible to be the canonical provider for agents. This excludes Goose (YAML recipes) and KimiCode (YAML with Markdown sidecar) from canonical selection. During `claudine init`, these providers must not appear as options for the agent canonical provider.

## Provider Support Matrix

| Provider  | Support Level  | Format   | Repo Path                  | User Path                       | Required Props                              | Optional Props                                                                           |
|-----------|---------------|----------|----------------------------|---------------------------------|---------------------------------------------|------------------------------------------------------------------------------------------|
| Claude    | Full          | Markdown | `.claude/agents`           | `.claude/agents`                | `name`, `description`                       | `tools`, `disallowedTools`, `model`, `permissionMode`, `maxTurns`, `skills`, `mcpServers`, `hooks`, `memory` |
| Codex     | Full          | Markdown | `.codex/agents`            | `.codex/agents`                 | (none)                                      | (none)                                                                                   |
| Gemini    | CustomFormat  | Markdown | `.gemini/agents`           | `.gemini/agents`                | `name`, `description`                       | `kind`, `tools`, `model`, `temperature`, `max_turns`, `timeout_mins`                     |
| Goose     | CustomFormat  | YAML     | `.goose/recipes`           | `.config/goose/recipes`         | `title`, `description`                      | `instructions`, `prompt`, `extensions`, `parameters`, `sub_recipes`                      |
| KimiCode  | CustomFormat  | YAML     | `.kimi/agents`             | `.kimi/agents`                  | `name`, `system_prompt_path`, `tools`       | `extend`, `system_prompt_args`, `exclude_tools`, `subagents`                             |
| OpenCode  | Full          | Markdown | `.opencode/agents`         | `.config/opencode/agents`       | `description`                               | `mode`, `model`, `temperature`, `top_p`, `tools`, `permission`, `steps`, `color`, `hidden`, `disable`, `prompt` |
| Qwen      | Full          | Markdown | `.qwen/agents`             | `.qwen/agents`                  | `name`, `description`                       | `tools`, `color`                                                                         |

## Reporting Sections

The reporting is broken down into the following sections:

1. Header Intro

   The initial four lines reported are always the same:

   - line 1: _blank line_
   - line 2: `<blue><b>Agents</b></blue>`
   - line 3: `<blue>==================</blue>`
   - line 4: _blank line_

   We then report on the **canonical** base providers:

   - the _canonical_ base providers will be defined in the user and repo configuration files and are set when the user runs `claudine init` (via an interactive Q and A).
       - obviously if the current working directory is **not** a git repo then we only report on the user scoped canonical provider
   - to provide symbolic links _to_ agents we need to isolate which provider will _provide_ the agent sources ... the "canonical provider" is the designated provider of agents.
   - **Only Markdown-based providers** are eligible as canonical for agents (see Key Differences #5). YAML-based providers (Goose, KimiCode) are excluded because cross-format conversion is not yet supported.
   - based on this context here are two examples of what **line 5** of the Header Intro section might look like:
       - example 1 (user & repo): `<blue><b>Canonical Providers:</b></blue> user: <b>{user-provider}</b>, repo: <b>{repo-provider}</b>`
       - example 2 (user only): `<blue><b>Canonical Providers:</b></blue> user: <b>{user-provider}</b>`
   - If a canonical provider is not configured, the value is shown as `<i><red>not configured</red></i>`

2. Defined Agents

   Within the Defined Agents area we have three distinct ways of displaying this content:

      - **Detail View**
          - Shown when there is exactly 1 agent being shown (typically due to a filter condition)
          - Whether the `-v` / `--verbose` flag was used has no effect
          - The first line of reporting on the agent is the agent name (bold, OSC8 link to the agent file) followed by the badge for the scope
          - The second line is the description of the agent (dim, italics) with word wrapping
          - Then a blank line
          - Now we show the **agent file content preview**: read the agent file and display up to 20 lines of its content using a `CodeBlock`-style rendering (or equivalent)
              - For Markdown agents: show the frontmatter block and the first few lines of body content
              - For YAML agents (Goose, KimiCode): show the first 20 lines of the YAML file
              - Left margin of 2 characters
          - If the agent specifies a `model` property, append a warning line after the content preview:
            `<dim><i><orange>note:</orange> this agent specifies a <b>model</b> property which limits cross-provider shareability</i></dim>`
            (See [Non-Portable Assets](non-portable-assets.md) for why `model` blocks sharing.)

      - **Verbose**
          - If the number of agents (_after filtering_) is less than 6 (and more than 1) we will report using the verbose style.
          - If the user adds the `--verbose` or `-v` flag and there is more than 1 agent then we will also report using the verbose style.
          - This mode lists all agents available (after filter) as an unordered list (leveraging `UnorderedList` component from biscuit-terminal)
              - Each item shows: OSC8-linked agent name (bold), scope badge, format badge (see below), then description (dim, italics)
          - The **format badge** is a small indicator showing the agent's file format:
              - Markdown: `<dim>[md]</dim>`
              - YAML: `<dim>[yaml]</dim>`
              - This helps the user quickly identify which agents may have linking complications

      - **Normal**
          - When we have more than 5 agents (and verbose is not forced) we group agents by scope
          - Grouping uses `BTreeMap<AgentScope, Vec<&AgentInfo>>` so scopes appear in order: User, RepoMasked, Repo
          - Each scope section leads with the scope badge and a count: `{badge} <dim>(<i>{count}</i>)</dim>`
          - Followed by a blank line
          - Then agent names rendered as space-separated OSC8 links (bold) with `WordWrap::BespokeProse(Some(50), ...)` to flow across the terminal width
          - Followed by another blank line

3. Fix Summary

   - This section is only shown when `--apply` / `--fix` was used
   - Rendered after the Defined Agents section
   - Shows: `<b>Fix Summary</b>` header then a dim comma-separated metric line:
     `directories_created={n}, links_created={n}, already_linked={n}, skipped={n}, aliases_inserted={n}, aliases_resynced={n}, format_incompatible={n}`
   - `aliases_inserted` counts alias properties added to canonical files (e.g., adding `title` alongside `name`)
   - `aliases_resynced` counts diverged alias values that were corrected to match their canonical property
   - `format_incompatible` counts agents that could not be linked due to format mismatch (e.g., Markdown canonical source targeting a YAML-only provider)

4. Auto-Init Behavior

   - When `--apply` is used in a git repo and the repo canonical provider is not configured, claudine automatically runs `claudine init --repo` before proceeding with the fix operation.

5. Auto Fixes

   When `--apply` / `--fix` is used, claudine can automatically resolve certain exceptions without human intervention. Agents have the richest set of property aliases across providers, making auto-fix particularly valuable here.

   **Symlink fixes:**

   - **Create missing directories**: If a target provider's agent directory does not exist (but its parent does), create it.
   - **Create missing symlinks**: For each canonical agent that has no corresponding entry in a compatible target provider's directory, create a symlink pointing back to the canonical source.

   **Property aliasing (enrich canonical, keep symlinks):**

   CLIs ignore frontmatter properties they don't recognize. This means we can add _all_ alias variants directly into the canonical source file and continue to symlink everywhere. This is a one-time enrichment of the canonical file — after which symlinks keep everything in sync by definition, avoiding the drift risk of derived copies.

   For example, if the canonical agent has `name: Code Review`, the fix adds `title: Code Review` to the same frontmatter block. The file now satisfies Claude (which reads `name`) and Goose (which reads `title`) — both through the same symlink.

   Known property aliases:

   | Canonical Property | Alias Property      | Target Provider | Derivation                                                   |
   |--------------------|---------------------|-----------------|--------------------------------------------------------------|
   | `name`             | `title`             | Goose           | Copied as-is — same semantic meaning                         |

   The fix inserts missing aliases and tracks them as `aliases_inserted` in the Fix Summary.

   **Drift detection — `VariantLinkedProperty` exception:**

   After aliases are established, they should always carry the same semantic value. If a user later edits `name` but forgets to update `title` (or vice versa), the values diverge. Claudine detects this and reports a `VariantLinkedProperty` exception:

   `<b>{agent_name}</b> (<i><orange>name</orange>="{value_a}" differs from <orange>title</orange>="{value_b}"</i>)`

   The `--apply` flag can re-sync diverged aliases by copying the canonical property's value to its aliases. The canonical property is determined by provider priority: the property that the canonical provider actually reads is authoritative.

   **Property passthrough:**

   Many properties — `temperature`, `top_p`, `max_turns`, `timeout_mins`, `tools`, `permissions`, etc. — are only recognized by some CLIs. However, under the same simplifying assumption that drives the alias strategy (extra properties cause no downside to CLIs that don't use them), these values can live in the canonical file and pass through symlinks harmlessly. CLIs that understand a given property will use it; those that don't will ignore it. For the full analysis of which properties are safe to pass through and which block sharing, see [Non-Portable Assets](non-portable-assets.md).

   When the `--verbose` flag is used, the Footer Messages section includes per-property notes showing which CLIs actually consume each property present in the listed agents (see section 7).

   **Fixes that cannot be automated:**

   - **Format conversion** (Markdown → YAML for Goose, KimiCode): Structural differences between Markdown frontmatter and YAML schemas are too significant for automated conversion. Tracked as `format_incompatible`.
   - **`model` property translation**: Provider-specific. Planned for Phase 2 (see `model-property-design.md`). In Phase 1, flagged as `ModelPropertyNotShareable`. See [Non-Portable Assets](non-portable-assets.md) for the full portability analysis of `model`, `tools`, and `skills`.
   - **`system_prompt_path` for KimiCode**: KimiCode agents require a file path to an external system prompt. This has no equivalent in other providers and cannot be derived.

6. Exceptions

   - This area is only shown if there **are** exceptions (either `AgentException` entries or `AgentDirectoryDiagnostic` entries)
   - The exceptions section does NOT include explicit `--fix` callouts; the Footer Messages section handles that
   - Exceptions are rendered in two groups: **format-incompatible providers** and **regular providers**

   **Format-incompatible providers** are rendered as simple one-liners in an unordered list, with no sub-bullets:
   - `<b>{provider}</b> — ❌ uses a non-standard format which is incompatible with Claudine.`
   - Examples: Goose (YAML recipes), KimiCode (YAML)

   **Regular providers** are grouped by provider with a header line showing provider name, user agent path, and repo agent path:
   - `<b>{provider} [ user:</b> ~/{user_path}<b>, repo:</b> <magenta>{repo_path}</magenta> ]`

   **Directory-level diagnostics** (missing agent directories) are rendered directly at the provider level, NOT nested under a "missing" category:
   - `All <b><yellow>{count}</yellow> {scope}</b> scoped agents are missing for <b>{provider}</b> because the directory for agents doesn't exist!`
   - These appear as immediate children of the provider item in the unordered list

   Within each regular provider, exceptions are further grouped by `ExceptionType`:

     - **Missing**: Comma-separated list of missing agent names with word wrapping (directory-level diagnostics are shown separately above, not nested here)
     - **Invalid**: Each agent shown individually as an OSC8 link with missing property details: `<b>{agent_name}</b> (<i>missing the properties <red>{prop1}</red>, <red>{prop2}</red></i>)`
     - **NoLinks**: Comma-separated OSC8-linked agent names with word wrapping
     - **ModelPropertyNotShareable**: Agents that specify a `model` property are flagged here. Each agent shown as an OSC8 link: `<a href="{path}"><b>{agent_name}</b></a> (<i>specifies <orange>model</orange></i>)`
     - **VariantLinkedProperty**: Alias properties that have diverged from their canonical value. Each entry shows: `<b>{agent_name}</b> (<i><orange>{canonical_prop}</orange>="{canonical_value}" differs from <orange>{alias_prop}</orange>="{alias_value}"</i>)`. Auto-fixable with `--apply`.

   - Note: The `BrokenLink` exception type from skills is **not applicable** to agents. Since agents are single files rather than directory bundles with inter-file links, broken internal links are not a concern.
   - Exceptions use the same _filtering_ rules as the Defined Agents section so we should ONLY report on those agents which match the fuzzy matching of the filter globs passed in
   - When filters are active, diagnostics are cleared entirely

7. Footer Messages

   This section is optionally rendered, it depends on whether the current _state_ dictates that additional context should be provided to the user. The following are messages that _might_ be shown (including an explanation of when they should be):

   - **fix**
       - the message `<dim><i>use <red>--fix</red> to attempt to fix the reported issues</i></dim>`
       - only shown when there are exceptions being reported on AND `--apply` was NOT used
   - **user only**
       - the message `<dim><i>the current working directory is <b>not</b> a <b>git</b> repo so we are only showing user-based scope</i></dim>`
       - only shown when the CWD is not inside a git repo
   - **verbose**
       - the message `<dim><i>using the <green>--verbose</green> switch will provide not only agent names but also descriptions and format indicators</i></dim>`
       - only shown when there are more than 10 agents listed and the user has not used the `--verbose`/`-v` flag
   - **filtering**
       - the message `<dim><i>using parameters in the CLI call will act as <b>filters</b> to help reduce the agents to only those you are interested in</i></dim>`
       - only shown when no filter parameters were provided
   - **format warning**
       - the message `<dim><i>agents with <orange>CustomFormat</orange> providers (Goose, KimiCode) require format conversion and cannot be directly symlinked</i></dim>`
       - only shown when there is at least one `FormatIncompatible` exception being reported
   - **property passthrough notes** (verbose only)
       - only shown when the `-v` / `--verbose` flag is used AND at least one agent in the listing has properties that are only consumed by a subset of CLIs
       - each such property that appears across the listed agents gets its own line indicating which CLIs consume it:
           - `<b><yellow>temperature</yellow></b><dim> used by Gemini and OpenCode; other CLI Agents will ignore</dim>`
           - `<b><yellow>top_p</yellow></b><dim> used by OpenCode; other CLI Agents will ignore</dim>`
           - `<b><yellow>maxTurns</yellow></b><dim> used by Claude (<i>maxTurns</i>) and Gemini (<i>max_turns</i>); other CLI Agents will ignore</dim>`
           - `<b><yellow>timeout_mins</yellow></b><dim> used by Gemini; other CLI Agents will ignore</dim>`
       - only the properties that actually appear in the currently listed agents are shown (not all four unconditionally)

	If only a single message is to be displayed then it should just be displayed "as is" (indented with a leading space) with a leading blank line to separate it from the sections above.

	If _more_ than one message is to be displayed then the messages should be added to an `UnorderedList` struct. The leading blank line should be added in this use-case too.

## Data Model

The agents command introduces the following types (parallel to the skills types):

### AgentInfo

```rust
pub struct AgentInfo {
    /// Agent name derived from filename (without extension)
    pub name: String,
    /// Description extracted from frontmatter or first heading
    pub description: Option<String>,
    /// Scope: User, RepoMasked, or Repo
    pub scope: AgentScope,
    /// Absolute path to the agent file
    pub agent_file_path: PathBuf,
    /// File format (Markdown, Yaml)
    pub format: ResourceFormat,
    /// Provider that owns this agent definition
    pub provider: Provider,
    /// Whether the agent specifies a `model` property
    pub has_model_property: bool,
    /// Whether the agent specifies `temperature` or `top_p`
    pub has_numeric_tuning: bool,
}
```

### AgentScope

Reuses the same scope enum as skills:

```rust
pub enum AgentScope {
    /// User-level agent (~/.claude/agents, ~/.config/opencode/agents, etc.)
    User,
    /// Repo-level agent that masks a user-level agent with the same name
    RepoMasked,
    /// Repo-level agent with no user-level equivalent
    Repo,
}
```

### AgentException

```rust
pub struct AgentException {
    /// Agent name
    pub name: String,
    /// Provider where the exception was detected
    pub provider: String,
    /// Type of exception
    pub exception_type: AgentExceptionType,
    /// Path to the agent file (if it exists)
    pub agent_file_path: PathBuf,
    /// Missing required properties (for Invalid type)
    pub missing_properties: Vec<String>,
    /// Source format (for FormatIncompatible type)
    pub source_format: Option<ResourceFormat>,
    /// Target format (for FormatIncompatible type)
    pub target_format: Option<ResourceFormat>,
}
```

### AgentExceptionType

```rust
pub enum AgentExceptionType {
    /// Agent exists in canonical provider but not in this provider
    Missing,
    /// Agent file exists but is missing required properties
    Invalid,
    /// Agent has no symlinks to other providers
    NoLinks,
    /// Agent specifies a `model` property (not shareable)
    ModelPropertyNotShareable,
    /// Alias properties have diverged from their canonical value
    VariantLinkedProperty,
    /// Source format is incompatible with target provider format
    FormatIncompatible,
}
```

### AgentFixSummary

```rust
pub struct AgentFixSummary {
    pub directories_created: usize,
    pub links_created: usize,
    pub already_linked: usize,
    pub skipped: usize,
    pub aliases_inserted: usize,
    pub aliases_resynced: usize,
    pub format_incompatible: usize,
}
```

## Linking Behavior

### Same-Format Linking (Markdown to Markdown)

When the canonical source and target provider both use Markdown agents, linking works identically to skills: create a symlink from the canonical agent file to the target provider's agent directory.

Example: Claude canonical `~/.claude/agents/researcher.md` links to `~/.qwen/agents/researcher.md`.

### Cross-Format Linking (Markdown to YAML or vice versa)

When the canonical source format does not match the target provider's expected format, the agent **cannot be directly symlinked**. Instead:

- The agent is flagged with a `FormatIncompatible` exception
- The `--fix` operation increments the `format_incompatible` counter
- A future enhancement may support format conversion (Markdown frontmatter to YAML and vice versa), but this is out of scope for the initial implementation

## Agent Discovery

Agent discovery follows the same pattern as skill discovery but adapted for single files:

1. Enumerate all configured provider agent directories (both user and repo scope)
2. For each directory, scan for agent files matching the provider's expected format:
   - Markdown providers: `*.md` files
   - YAML providers: `*.yaml` and `*.yml` files
3. Parse each agent file to extract name, description, and properties
4. Determine scope (User, RepoMasked, Repo) based on path location
5. Flag agents with non-portable properties (`model`, `tools`, `skills`) as having limited shareability — see [Non-Portable Assets](non-portable-assets.md)
