# System Prompt Handling

The system-prompt refactor moved prompt resolution into a shared library pipeline and turned provider delivery into a launch-plan mutation step. Claudine now resolves a file-backed system prompt once, composes it through Darkmatter, and then lets each wrapped provider apply it with its own runtime strategy.

## Command Surfaces

System prompt handling is available on all wrapped provider subcommands:

- `claudine claude`
- `claudine codex`
- `claudine gemini`
- `claudine kimi`
- `claudine qwen`
- `claudine opencode`
- `claudine goose`

The same flags are also shared by all composition entry points because `compose`, `inline-compose`, and `sequence` all flow through the same wrapper-grade execution path:

- `claudine compose <file>`
- `claudine inline-compose <file>`
- `claudine sequence <file>`

## CLI Contract

Claudine no longer uses the old universal `--system-prompt <PROMPT|FILE>` switch on these paths. The current interface is:

- `--append-system-prompt <FILE>`
- `--replace-system-prompt <FILE>`
- visible aliases: `--asp` and `--rsp`

Behavior:

- both switches are file-only
- they are mutually exclusive
- explicit files are resolved as plain paths relative to the launch CWD unless already absolute
- if an explicit file is selected, standard `system-prompt.md` discovery is skipped

Internally these switches map to `claudine::system_prompt::SystemPromptArgs`.

## Shared Pipeline

The library pipeline lives in `claudine/lib/src/system_prompt/`:

1. `LaunchContext::from_cwd()` detects the launch workspace from the directory Claudine was started in
2. `resolve_system_prompt_source()` picks either an explicit file or a discovered `system-prompt.md`
3. `prepare_system_prompt()` composes the selected file through Darkmatter
4. providers apply the prepared result through `WrapperProfile::apply_system_prompt()`

`EffectiveSystemPrompt` is the handoff type between resolution/preparation and runtime delivery:

- `None` means no file was found or specified
- `Disabled` means a file was found, but its composed body was empty
- `Ready` contains the final `PreparedSystemPrompt`

## Discovery Rules

When no explicit flag is given, Claudine searches for a standard `system-prompt.md` based on the launch CWD, not the composition source file path.

Inside a detected repo/monorepo the search order is:

1. package root
2. package-area root
3. repo root
4. `~/.claudine/system-prompt.md`

Outside a detected repo the local search collapses to:

1. current working directory
2. `~/.claudine/system-prompt.md`

The standard discovered file is always treated as append-mode.

`LaunchContext` is built with `sniff` repo-structure detection and carries:

- `cwd`
- `repo_root`
- `package_area_root`
- `package_root`

## Composition Semantics

Selected prompt files are composed with Darkmatter before they ever reach the provider. That means the system prompt supports the same document-level composition features as other Claudine Markdown flows, including source-aware transclusion and interpolation.

Current preparation behavior:

- the source file path is passed into `ComposeOptions::with_source_file(...)`
- frontmatter is not forwarded to the provider
- the canonical output is Markdown as authored after composition
- if the composed body is empty or whitespace-only, Claudine treats that as an explicit disable for the selected scope

Important disable rule:

- an empty composed body stops the search and produces `EffectiveSystemPrompt::Disabled`
- Claudine does not continue to lower-priority `system-prompt.md` locations after that

## Provider Delivery

After preparation, each provider mutates its launch plan with args, env vars, and temp artifacts. Temporary files and directories are held alive until the child process exits.

| Provider | Append | Replace | Runtime strategy |
|---|---|---|---|
| Claude Code | Yes | Yes | Interactive uses native string flags; non-interactive writes temp files and uses `--append-system-prompt-file` or `--system-prompt-file` |
| Codex | Yes | Yes | Append uses an ephemeral `HOME` with `.codex/AGENTS.override.md`; replace uses `-c model_instructions_file=<temp>` |
| Gemini CLI | Yes | Yes | Append uses an ephemeral `HOME` with `.gemini/GEMINI.md`; replace sets `GEMINI_SYSTEM_MD=<temp>` |
| Kimi Code | No | Yes | Replace writes a temp prompt file plus a temp agent YAML and passes `--agent-file` |
| Qwen Code | Yes | No | Append uses an ephemeral `HOME` with `.qwen/QWEN.md` |
| OpenCode | Yes | Yes | Append sets `OPENCODE_CONFIG_CONTENT` with a temp instruction file; replace passes `--system <temp>` |
| Goose | Yes | No | Append passes `--system <markdown>` directly |

Unsupported modes are skipped with warnings rather than hard failures.

## Overlay-Home Providers

Codex, Gemini, and Qwen append-mode use an ephemeral overlay home instead of mutating the user's real home directory.

Current behavior:

- a temp home is created
- the provider subdirectory is created inside it
- if the real overlay file already exists, Claudine copies its contents and appends the composed prompt
- otherwise Claudine writes only the composed prompt
- `HOME` is pointed at the temp home for the launched child process

This preserves the user's real config while still letting the provider load its normal memory-file mechanism.

## Harness Integration

The system prompt capability model also informs wrapper-harness source discovery. `find_wrapper_harness_source()` looks at the selected agent's `runtime.system_prompt.memory_files`, ignores home-relative entries such as `~/.gemini/GEMINI.md`, and searches the repo root or current working directory for the first provider-specific memory file that exists on disk.

This is separate from Claudine-managed prompt injection:

- provider memory files remain a provider-native signal
- `system-prompt.md` is Claudine's standard discovery surface
- both can coexist in the same wrapped session
