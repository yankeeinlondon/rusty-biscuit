# Non-Portable Assets

While **skills** were such a hit with the public that all of the properties found in a skill are _largely_ 100% portable between Agentic CLIs. The same statement can not be made for **slash commands** and **agent/subagent definitions**.

## Non-Portable Properties

These three frontmatter properties will prevent a resource from being symlinked into other provider directories. The danger is not simply that they are "Claude-only" -- in some cases the property _name_ is shared across providers but the _value format_ is incompatible.

### `model`

The most common incompatibility between **commands** and **agents** is that they define a specific _model_ to use. This can be useful but the issue here is that each CLI has different models available to it. It's true that CLI Agents like **Opencode** can use virtually any model but even there, the way the model's are described can vary between the CLI's.

The property name `model` appears in the schemas of Claude, Gemini, and OpenCode -- but the values are provider-specific. Claude uses shorthand names like `sonnet`, `opus`, `haiku`; Gemini expects full model identifiers like `gemini-2.5-pro`; OpenCode uses provider-prefixed paths like `anthropic/claude-sonnet`. Symlinking a file that says `model: sonnet` into an OpenCode directory would either be silently wrong or fail validation.

The easiest and most prudent solution is to mark any shared asset (skill, command, agent defn) that has defined a specific model as non-portable.

#### Future Feature

We might in the future add an abstraction layer like:

- Smart(_thinking-level_)
- Moderate(_thinking-level_)
- Fast
- Cheap

Where models can be described by capability. This would allow users to move from their proprietary language to this abstracted language and gain portability.

### Tool Properties

Claude Code has three frontmatter properties that all serve the same conceptual purpose -- restricting which tools are available during execution. They landed on different names because agents and skills/commands were designed as separate systems within Claude Code and converged on the same idea independently.

| Property | Resource Type | Example | Portable? |
|---|---|---|---|
| `tools` | Agents / subagents | `tools: Bash, Read` | **No** |
| `disallowedTools` | Agents / subagents | `disallowedTools: Write` | Yes |
| `allowed-tools` | Skills, slash commands | `allowed-tools: Bash(git:*), Read` | Yes |

#### `tools` (non-portable)

The property name `tools` is shared across multiple providers -- Claude, Gemini, KimiCode (where it's _required_), OpenCode, and Qwen all declare it in their agent schemas. But the value formats are incompatible. Claude Code writes a comma-separated string of tool names (e.g., `tools: Bash, Read`), while OpenCode expects `tools` to be a structured record (a YAML mapping). Passing a Claude-style string value where a record is expected will crash the provider at config load time -- this is exactly what broke OpenCode when `claudine agents --fix` symlinked agent definitions containing `tools: Bash, Read` into `~/.config/opencode/agents/`.

#### `allowed-tools`, `allowed_tools`, `disallowedTools` (portable)

Claude Code uses `allowed-tools` (and its underscore variant `allowed_tools`) in **skills** and **slash commands** to restrict which tools are available while the resource is active. The syntax includes Claude-specific scoped permissions like `Bash(git:*)` or `Bash(cargo *)`. Similarly, `disallowedTools` serves as the inverse for **agents** -- a denylist of tools the subagent should not have access to.

Despite the Claude-specific value syntax, none of these properties are in the non-portable blocking list. The keys `allowed-tools` and `disallowedTools` do not appear in any other provider's declared schema for any resource type. Under the general simplifying assumption that Agentic CLI providers ignore frontmatter properties they don't recognize, these properties are inert when symlinked into foreign directories.

#### Why the difference?

The critical distinction is _name collision_. `tools` collides with a property name used by other providers that expect a different value format. `allowed-tools` does not collide with anything -- no other provider declares or attempts to parse a key by that name.

### `skills`

Claude Code lets agent definitions reference skills by name via a `skills` frontmatter field (e.g., `skills: rust, cli`). This tells Claude Code to auto-load the named skill context when the agent is invoked. No other provider has an equivalent mechanism. The property is unique to Claude Code and will be ignored by providers that silently skip unrecognized keys. However, we cannot guarantee all providers will ignore it gracefully, so it remains in the non-portable list as a precaution.

## The Blocking Rule

Any skill, command, or agent definition that contains **any** of these three frontmatter properties is considered _non-portable_ and will **not** be symlinked into other provider directories by `claudine --fix`:

| Property | Typical Usage | Why It Breaks |
|---|---|---|
| `model` | `model: sonnet` | Shared property name but provider-specific values |
| `tools` | `tools: Bash, Read` | Shared property name but incompatible value formats (string vs record) |
| `skills` | `skills: rust, cli` | Claude-only; no equivalent mechanism in other providers |

The check is implemented centrally in `linking/compatibility.rs` as `NON_PORTABLE_PROPERTIES` and applied by all three fix paths (agents, commands, skills) before any symlink creation occurs. Resources that fail this check are counted in the `not_shareable` field of the fix summary.

## Format Incompatibilities

Beyond frontmatter properties, some providers use a fundamentally different file format for their resources. A Markdown file cannot be symlinked into a directory that expects TOML or YAML, regardless of what properties it contains. These show up as `FormatIncompatible` exceptions and are filtered out before the property-level portability check is reached.

### Serialization Format vs Schema

The format difference is not just about serialization (YAML vs Markdown frontmatter). The YAML-based providers each define **structurally different schemas** with different required properties, different property names, and different value types. The YAML that these providers expect is not the same data as Markdown frontmatter written in YAML syntax -- it is a different schema entirely.

### Gemini — TOML Commands

Gemini CLI stores commands as `.toml` files rather than Markdown. The required properties are `name` and `description`, matching the Markdown convention in name only. The `prompt` field in TOML serves the role that the Markdown body plays in Claude Code commands.

### Goose — YAML "Recipes"

Goose stores agent definitions as YAML files called "recipes" in `.goose/recipes/` (repo) or `.config/goose/recipes/` (user).

| Goose Property | Required | Claude Equivalent | Notes |
|---|---|---|---|
| `title` | Yes | `name` | Different property name |
| `description` | Yes | `description` | Same name, same purpose |
| `instructions` | No | body content | Free-form guidance text |
| `prompt` | No | body content | Alternative to `instructions` |
| `extensions` | No | `tools` | Goose's term for tool/plugin references |
| `parameters` | No | _(none)_ | No Claude equivalent |
| `sub_recipes` | No | _(none)_ | Nested recipe composition; no Claude equivalent |

A Claude agent definition with `description: "Code reviewer"` in Markdown frontmatter cannot become a Goose recipe by simply re-serializing as YAML -- the recipe requires `title` (not `name`), and Goose ignores `description` if `title` is absent.

### KimiCode — YAML Agents

KimiCode stores agent definitions as YAML files in `.kimi/agents/`.

| KimiCode Property | Required | Claude Equivalent | Notes |
|---|---|---|---|
| `name` | Yes | `name` | Same name |
| `system_prompt_path` | **Yes** | _(none)_ | File path to an external prompt document; no Claude equivalent |
| `tools` | **Yes** | `tools` | Same property name but **structured YAML mapping**, not a comma-separated string |
| `extend` | No | _(none)_ | Inheritance mechanism; no Claude equivalent |
| `system_prompt_args` | No | _(none)_ | Template variables for the system prompt |
| `exclude_tools` | No | `disallowedTools` | Similar concept, different name |
| `subagents` | No | _(none)_ | No Claude equivalent |

KimiCode agents are fundamentally incompatible because two of the three required properties (`system_prompt_path` and `tools` as a structured type) have no Claude equivalent and cannot be derived from any information in a Claude agent definition.

### Why Automated Format Conversion Is Not Viable

The `link` command's derived artifact workflow attempts to bridge the format gap by converting Markdown frontmatter to TOML or YAML. However, this conversion is purely a _serialization_ change -- it copies Claude's frontmatter keys into the target format and maps the body to a `prompt` field. It does not:

- Translate property names (`name` → `title` for Goose)
- Generate required properties that have no source (`system_prompt_path` for Kimi)
- Convert value formats (`tools: Bash, Read` as a string → structured YAML mapping for Kimi)

The resulting files would fail schema validation for every YAML-based provider. This means the derived artifact workflow produces non-functional output for agent definitions, and the `FormatIncompatible` exception raised by `skills --apply`, `agents --apply`, and `commands --apply` is the correct behavior.

For **Gemini TOML commands**, the conversion is closer to viable because Gemini's command schema is structurally similar to Claude's (both use `name`, `description`, and a `prompt` field). However, even here, the conversion is limited to resources that do not contain non-portable properties.

## Required Property Gaps

Even among providers that _do_ use Markdown, their schemas may require properties that the canonical source does not provide. For example, KimiCode requires `system_prompt_path` and `tools` (as a structured type) that Claude Code definitions lack entirely. When a target provider requires a property that the source file does not contain, linking is blocked and an `Invalid` / `MissingProperties` exception is raised.

## Exception Categories

Claudine tracks portability issues as typed exceptions. Each resource type (skill, command, agent) has its own exception enum, but the categories are largely parallel.

### Shared Across Agents and Commands

| Exception | Display Name | Meaning |
|---|---|---|
| `Missing` | `missing` | The resource exists in the canonical provider but has no corresponding file (or symlink) in the target provider's directory. Fixable by `--fix`. |
| `Invalid` | `invalid` | The resource file exists but is missing required frontmatter properties (e.g., `description`). Not auto-fixable. |
| `NoLinks` | `no-links` | The resource body is non-trivial but contains no markdown links. This is a quality warning -- skills and commands that reference supporting documents should link to them. |
| `ModelPropertyNotShareable` | `model-not-shareable` | The resource defines a `model` property. Reported as a warning during listing to flag limited shareability. |
| `FormatIncompatible` | `format-incompatible` | The target provider uses a non-Markdown format (TOML, YAML, MCP) for this resource type. Symlinking a Markdown file would be meaningless or harmful. |

### Skills Only

| Exception | Display Name | Meaning |
|---|---|---|
| `Missing` | `missing` | Same as above. |
| `Invalid` | `invalid` | SKILL.md is missing the required `description` frontmatter. |
| `BrokenLink` | `broken-link` | SKILL.md contains a relative markdown link that does not resolve to an existing file. This typically means a supporting document was renamed or deleted. |
| `NoLinks` | `no-links` | SKILL.md body exceeds a trivial length but contains no markdown links. Large skills should reference detail documents rather than inlining everything. |

### Fix Summary Counters

When `--fix` is run, the operation summary reports:

| Counter | Meaning |
|---|---|
| `directories_created` | Provider directories that were created because they didn't exist. |
| `links_created` | Symlinks successfully created for missing resources. |
| `already_linked` | Resources that were already correctly linked. |
| `skipped` | Resources skipped for other reasons (e.g., a real directory already exists at the target path, or the symlink points somewhere unexpected). |
| `format_incompatible` | Resources not linked because the target provider uses a different file format. |
| `not_shareable` | Resources not linked because they contain non-portable frontmatter properties (`model`, `tools`, `skills`). |
| `names_inserted` | (Skills only) SKILL.md files that had a missing `name` property auto-inserted from the directory name. |

## Practical Impact

In a typical Claude Code user configuration, the ratio of non-portable to portable resources is heavily skewed. For example, in a setup with 27 agent definitions, only 3 (those with just `description` and `name`) were shareable -- the other 24 all specified `model`, `tools`, or `skills`. Commands fare better now that `allowed-tools` is no longer in the blocking list -- many commands that only use `allowed-tools` (like `commit`, `dependencies`, review commands) are now shareable.

This means the `--fix` command will intentionally leave most _agent_ definitions unlinked for non-Claude providers, while a larger proportion of commands and skills will link successfully. That is the correct behavior. A broken provider is far worse than a missing agent definition.
