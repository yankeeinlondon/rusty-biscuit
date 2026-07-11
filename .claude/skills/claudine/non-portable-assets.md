# Non-Portable Assets

While skills are largely portable between agentic CLIs, slash commands and agent/subagent definitions often contain frontmatter properties that prevent cross-provider symlinks.

## Blocking Properties

Three frontmatter properties mark a resource as non-portable:

| Property | Example | Why It Breaks |
|----------|---------|---------------|
| `model` | `model: sonnet` | Shared property name but provider-specific values (Claude: `sonnet`/`opus`; Gemini: `gemini-2.5-pro`; OpenCode: `anthropic/claude-sonnet`) |
| `tools` | `tools: Bash, Read` | Shared property name but incompatible value formats (Claude: comma-separated string; OpenCode/Kimi: structured YAML mapping) |
| `skills` | `skills: rust, cli` | Claude-only mechanism for auto-loading skill context; no equivalent in other providers |

The check is in `linking/compatibility.rs` as `NON_PORTABLE_PROPERTIES`, applied by all three fix paths (agents, commands, skills) before symlink creation.

## Properties That Are NOT Blocking

- `allowed-tools` / `allowed_tools` -- Claude-specific key name, no collision with other providers (they ignore it)
- `disallowedTools` -- same, unique to Claude Code agents

The critical distinction is **name collision**: `tools` collides with a property other providers parse differently. `allowed-tools` does not appear in any other provider's schema.

## Format Incompatibilities

Some providers use fundamentally different file formats, blocked before property checks:

| Provider | Format | Required Properties | Notes |
|----------|--------|---------------------|-------|
| **Gemini** | TOML commands | `name`, `description` | Closest to viable conversion; `prompt` field replaces body |
| **Goose** | YAML recipes | `title` (not `name`), `description` | Different property names; `extensions` instead of `tools` |
| **KimiCode** | YAML agents | `name`, `system_prompt_path` (required), `tools` (structured, required) | Two required properties have no Claude equivalent |

These surface as `FormatIncompatible` exceptions and are filtered before property-level checks.

## Required Property Gaps

Even among Markdown providers, target schemas may require properties the canonical source lacks. Missing required properties produce `Invalid` / `MissingProperties` exceptions.

## Exception Categories

Shared across agents and commands: `Missing` (fixable), `Invalid` (not auto-fixable), `NoLinks` (quality warning), `ModelPropertyNotShareable` (warning), `FormatIncompatible`.

Skills only: `Missing`, `Invalid`, `BrokenLink`, `NoLinks`.

## Fix Summary Counters

`--fix` reports: `directories_created`, `links_created`, `already_linked`, `skipped`, `format_incompatible`, `not_shareable`, `names_inserted` (skills only).

## Practical Impact

In typical setups, most agent definitions are non-portable (specify `model`, `tools`, or `skills`). Commands fare better since `allowed-tools` is not blocking. Skills have the highest portability. The `--fix` command intentionally leaves non-portable resources unlinked -- a broken provider is worse than a missing definition.
