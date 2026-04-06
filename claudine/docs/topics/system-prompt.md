# System Prompt Handling

Claudine provides unified `--append-system-prompt` (`--asp`) and `--replace-system-prompt` (`--rsp`) flags across all entry points (`wrap`, `compose`, `inline-compose`) as well as standard `system-prompt.md` file discovery.

## Entry Points

All three command surfaces accept the new flags:

| Command                           | Clap definition                                   | Internal type                          |
|-----------------------------------|-----------------------------------------------------|---------------------------------------|
| `claudine wrap <provider> [args]` | `WrapperArgs.append_system_prompt` / `replace_system_prompt` | `SystemPromptArgs`                          |
| `claudine compose <file>`         | `SharedComposeArgs.append_system_prompt` / `replace_system_prompt` | `SystemPromptArgs`                          |
| `claudine inline-compose <file>`  | `SharedComposeArgs.append_system_prompt` / `replace_system_prompt` | `SystemPromptArgs`                          |

Both flags accept file paths and absolute paths only. They are mutually exclusive.

 Short aliases: `--asp` and `--rsp`.

## Resolution Pipeline

The resolution pipeline (`claudine/lib/src/system_prompt/`) handles file discovery and Darkmatter composition:

1. **LaunchContext** — detects git root, package root, and package-area root from the CWD
2. **resolve_system_prompt_source()** — selects the source file based on CLI args or standard file discovery
3. **prepare_system_prompt()** — composes through Darkmatter, handles empty-body disable
4. **EffectiveSystemPrompt** — the final result: `None`, `Disabled`, or `Ready(PreparedSystemPrompt)`

### Standard File Discovery

When no explicit flags are given, Claudine searches for `system-prompt.md` in priority order:

1. Package root (inside a Cargo workspace package)
2. Package-area root
3. Repository root
4. User home (`~/.claudine/system-prompt.md`)

## Provider-Specific Application
Each provider translates the prepared system prompt into provider-specific CLI arguments and env vars via the `apply_system_prompt()` on the `WrapperProfile`:
```rust
fn apply_system_prompt(
    &self,
    prompt: &PreparedSystemPrompt,
    interactive: bool,
    cwd: &Path,
) -> Result<SystemPromptApplication>
```

The default implementation returns a warning. Providers that support it override the method.

### Provider Support Matrix
| Provider    | Append | Replace | Strategy                                                              |
|-------------|--------|---------|----------------------------------------------------------------------|
| Claude Code | Yes    | Yes     | Native CLI flags (`--append-system-prompt`, `--system-prompt`)             |
| Codex       | Yes    | Yes     | Append: ephemeral home + `AGENTS.override.md`. Replace: `-c model_instructions_file` |
| Gemini CLI  | Yes    | Yes     | Append: ephemeral home + `GEMINI.md`. Replace: `GEMINI_SYSTEM_MD` env |
| Goose       | Yes    | No      | Append: `--system`. Replace: warn + skip                             |
| Kimi Code   | No     | Yes     | Replace: temp agent YAML + `--agent-file`. Append: warn + skip     |
| OpenCode    | Yes    | Yes     | Append: temp file + `OPENCODE_CONFIG_CONTENT`. Replace: `--system` |
| Qwen CLI   | Yes    | No      | Append: ephemeral home + `QWEN.md`. Replace: warn + skip                |
### Ephemeral Overlay Home
For providers that need HOME override (Codex, Gemini, Qwen append), Claudine creates a temporary home directory with the provider config overlay file. If the user already has an overlay file (e.g. `AGENTS.override.md`), its existing content is preserved and the Claudine prompt is appended after it.

### Artifact Lifetime
Temp files and temp directories created during system prompt application must kept alive until the child process exits. Rust's RAII cleanup guarantees they are dropped after `child.wait()` completes.

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
## Harness Integration
The system prompt capability model is used at runtime by the harness to locate provider-specific memory files.
### Memory File Discovery
`find_wrapper_harness_source()` searches for the first existing non-home memory file from the agent's `system_prompt.memory_files` list:
1. Maps the provider to its `AgentId`
2. Reads `agent.capabilities().runtime.system_prompt.memory_files`
3. Filters out home-relative paths (those starting with `~`)
4. Searches from the repo root (or CWD) for the first file that exists on disk

