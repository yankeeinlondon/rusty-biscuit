# System Prompt Handling

Claudine provides a unified `--system-prompt` flag across all entry points (`wrap`, `compose`, `inline-compose`) that resolves a user-supplied value and translates it into provider-specific CLI arguments. The system also models each agent's native system prompt capabilities for harness integration and provider selection.

## Entry Points

All three command surfaces accept `-s` / `--system-prompt`:

| Command | Clap definition | Flows into |
|---|---|---|
| `claudine wrap <provider> [args]` | `WrapperArgs.system_prompt` | `run_wrapper()` in `wrap/mod.rs` |
| `claudine compose <file>` | `ComposeArgs.system_prompt` | `CompositionExecutionRequest.system_prompt` |
| `claudine inline-compose <file>` | `InlineComposeArgs.system_prompt` | `CompositionExecutionRequest.system_prompt` |

The value is always `Option<String>` — either a literal prompt string or a file path.

## Resolution

`resolve_system_prompt()` (`wrap/mod.rs:3024-3031`) handles the dual-mode input:

```rust
resolve_system_prompt(prompt_or_file: &str) -> Result<String>
```

1. If `prompt_or_file` is an existing file path → reads and returns the file contents
2. Otherwise → returns the string as-is (literal prompt)

This function is called identically in both the wrapper path (`wrap/mod.rs:806`) and the composition path (`wrap/composition.rs:369`).

## Provider-Specific Application

The resolved prompt is applied via the `WrapperProfile` trait method:

```rust
fn apply_system_prompt(&self, args: &mut Vec<String>, prompt: &str) -> Option<String>
```

The default implementation returns a warning string indicating the provider does not support `--system-prompt`. Providers that support it override the method and return `None` (no warning).

### Provider Implementations

| Provider | Supported | CLI mapping | Profile location |
|---|---|---|---|
| Claude Code | Yes | `--system-prompt <prompt>` | `profile.rs:387-391` |
| Codex | Default (warns) | — | trait default |
| Gemini CLI | Default (warns) | — | trait default |
| Goose | Default (warns) | — | trait default |
| Kimi Code | Default (warns) | — | trait default |
| OpenCode | Default (warns) | — | trait default |
| Qwen CLI | Default (warns) | — | trait default |
| Roo Code | Default (warns) | — | trait default |

Currently only Claude Code has an `apply_system_prompt` override. All other providers fall through to the default, which emits a warning and skips the flag.

When a warning is generated:
- In `wrap` mode: the warning is pushed to `deferred_warnings` and printed after execution header output
- In `compose` / `inline-compose` mode: the warning is logged via `log::warn()` unless `--quiet` or `--silent` is set

## Agent Capability Model

Each agent descriptor in `claudine/lib/src/agents/` declares a `SystemPromptCapabilities` struct:

```rust
pub struct SystemPromptCapabilities {
    pub supplement_sources: Vec<&'static str>,
    pub full_replacement_supported: bool,
    pub replacement_mechanisms: Vec<&'static str>,
    pub memory_files: Vec<&'static str>,
}
```

### Field Semantics

- **`supplement_sources`** — mechanisms that _append_ to the system prompt (e.g. `--append-system-prompt`, `CLAUDE.md hierarchy`, `AGENTS.md`)
- **`full_replacement_supported`** — whether the provider supports completely replacing the default system prompt
- **`replacement_mechanisms`** — CLI flags or config keys for full replacement (e.g. `--system-prompt`, `model_instructions_file`)
- **`memory_files`** — files the agent reads as part of its system prompt (e.g. `~/.claude/CLAUDE.md`, `AGENTS.md`)

### Per-Agent Capabilities

| Agent | Supplement sources | Full replacement | Replacement mechanisms | Memory files |
|---|---|---|---|---|
| Claude Code | `--append-system-prompt`, `--append-system-prompt-file`, `CLAUDE.md hierarchy` | Yes | `--system-prompt`, `--system-prompt-file` | `~/.claude/CLAUDE.md`, `CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/CLAUDE.local.md` |
| Codex | `AGENTS.md`, `AGENTS.override.md`, `developer_instructions` | Yes | `model_instructions_file` | `~/.codex/AGENTS.override.md`, `~/.codex/AGENTS.md`, `AGENTS.md` |
| Gemini CLI | `GEMINI.md hierarchy`, `/memory`, `@file imports` | Yes | `GEMINI_SYSTEM_MD` | `~/.gemini/GEMINI.md`, `.gemini/GEMINI.md`, `GEMINI.md` |
| Goose | `.goosehints`, `goose run --system`, `GOOSE_MOIM_MESSAGE_*`, `recipe instructions` | No | — | `.goosehints` |
| Kimi Code | `AGENTS.md via /init` | Yes | `--agent-file with system_prompt_path` | `AGENTS.md` |
| OpenCode | `AGENTS.md` | No | — | `AGENTS.md` |
| Qwen CLI | `QWEN.md hierarchy`, `@path markdown imports`, `/memory refresh` | No | — | `~/.qwen/QWEN.md`, `QWEN.md` |
| Roo Code | `rules directories`, `.roorules/.roorules-{mode}`, `AGENTS.md/AGENT.md`, `.rooignore` | Yes | `.roo/system-prompt-{mode-slug}` | `AGENTS.md`, `AGENT.md` |

## Harness Integration

The system prompt capability model is used at runtime by the harness to locate provider-specific memory files.

### Memory File Discovery

`find_wrapper_harness_source()` (`wrap/mod.rs:1441-1461`) searches for the first existing non-home memory file from the agent's `system_prompt.memory_files` list:

1. Maps the provider to its `AgentId`
2. Reads `agent.capabilities().runtime.system_prompt.memory_files`
3. Filters out home-relative paths (those starting with `~`)
4. Searches from the repo root (or CWD) for the first file that exists on disk

The discovered file is used as the harness source document — its frontmatter may contain handler definitions that drive the harness loop.

### Composition Harness Path

For `compose` and `inline-compose`, the harness uses `materialized_harness_prompt_from_prepared()` (`wrap/mod.rs:1387-1401`) which extracts:

- The composed prompt text
- The effective frontmatter (single source of truth)
- The inline closure plan (if inline mode)

These are used to construct a `MaterializedHarnessPrompt` that drives handler-based recovery and post-execution closure.

## Inline Composition and Guardrails

In inline composition mode, the system prompt is separate from the composed prompt. The composed prompt (derived from frontmatter `prompt` property) gets guardrails appended automatically:

1. The `prompt` frontmatter property is extracted and composed through Darkmatter
2. Default guardrails from `.claudine/inline-compose.md` (or built-in defaults) are appended
3. The guardrails instruct the agent to return body content only, without frontmatter

Default guardrails (`composition/guardrails.rs`):

```markdown
> **IMPORTANT:**
>
> - Return the replacement Markdown body content only
> - Do not include frontmatter delimiters or frontmatter content
> - Do not edit the source file directly
```

Users can customize guardrails by editing `.claudine/inline-compose.md` in the repo root. Claudine creates this file with the defaults if it doesn't exist.

The `--system-prompt` flag is applied _in addition to_ these guardrails — it is passed to the provider as a separate system prompt, not merged into the composed prompt.

## Closure: Frontmatter Preservation

After inline composition, the closure pipeline (`composition/closure.rs`) ensures the original frontmatter is never lost, regardless of what the provider returns:

1. `extract_replacement_body()` strips any accidental frontmatter fences from provider output
2. `apply_inline_closure()` validates the body is non-empty and differs from the original
3. `rewrite_inline_document()` reconstructs the file using the _original_ frontmatter with only `last_updated` changed

This means system prompt configuration stored in document frontmatter (e.g. `agent`, `prompt`, custom properties) survives provider execution intact.

## Key Source Files

| File | Role |
|---|---|
| `cli/src/commands/wrap/mod.rs` | `WrapperArgs`, `resolve_system_prompt()`, `find_wrapper_harness_source()` |
| `cli/src/commands/wrap/profile.rs` | `WrapperProfile` trait with `apply_system_prompt()` |
| `cli/src/commands/wrap/composition.rs` | Composition execution pipeline (system prompt application) |
| `cli/src/commands/compose.rs` | `ComposeArgs`, `InlineComposeArgs` (clap definitions) |
| `lib/src/agents/model.rs` | `SystemPromptCapabilities` struct |
| `lib/src/agents/*.rs` | Per-agent capability declarations |
| `lib/src/composition/prepare.rs` | Prompt preparation (direct and inline) |
| `lib/src/composition/closure.rs` | Post-execution frontmatter preservation |
| `lib/src/composition/guardrails.rs` | Inline composition guardrail loading |
