---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
docs: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
system_prompt_docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
append_support: native
replace_support: native
cli_params:
  - flag: "--system-prompt <TEXT>"
    mode: replace
    value_shape: "inline text"
    description: "Overrides the built-in main-session system prompt for the current run."
    example: 'qwen -p "Review this patch" --system-prompt "You are a terse release reviewer."'
    notes: "Documented for headless and listed in the general CLI argument table. It replaces the built-in main-session prompt only; loaded context and memory such as QWEN.md are still appended after it. Can be combined with --append-system-prompt."
  - flag: "--append-system-prompt <TEXT>"
    mode: append
    value_shape: "inline text"
    description: "Appends extra instructions to the main-session system prompt for the current run."
    example: 'qwen -p "Review this patch" --append-system-prompt "Focus on concrete findings."'
    notes: "Documented as applied after the built-in prompt and loaded memory/context. Can be combined with --system-prompt. There is no documented --append-system-prompt-file equivalent."
  - flag: "--prompt <TEXT> / -p <TEXT>"
    mode: other
    value_shape: "inline text"
    description: "Runs a non-interactive headless prompt, which is the documented mode for prompt override examples."
    example: 'qwen -p "Summarize this repository" --append-system-prompt "Return three bullets."'
    notes: "Not a system-prompt switch, but wrapper-level append/replace delivery normally pairs it with headless mode."
  - flag: "--prompt-interactive <TEXT> / -i <TEXT>"
    mode: other
    value_shape: "inline text"
    description: "Starts an interactive session with an initial user prompt."
    example: 'qwen -i "explain this code" --system-prompt "You are a careful tutor."'
    notes: "The prompt is processed within the interactive session. The source parser accepts the system-prompt flags globally, but official examples for system-prompt manipulation are headless."
  - flag: "--safe-mode"
    mode: disable
    value_shape: "boolean switch"
    description: "Disables customizations that can otherwise affect prompt/context: context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, memory features, sandbox settings, and settings-sourced approval overrides."
    example: 'qwen --safe-mode -p "query" --append-system-prompt "Be concise."'
    notes: "Also settable via QWEN_CODE_SAFE_MODE=true. It does not disable --system-prompt or --append-system-prompt."
  - flag: "--extension <LIST> / -e <LIST>"
    mode: modify
    value_shape: "comma-separated extension names; special value none"
    description: "Selects which extensions are loaded, indirectly affecting extension-provided context, skills, MCP servers, and subagents."
    example: 'qwen -e none -p "query"'
    notes: "Use -e none when a wrapper needs a more deterministic baseline and does not want extension-sourced prompt surfaces."
  - flag: "--include-directories <PATHS>"
    mode: modify
    value_shape: "comma-separated paths"
    description: "Adds directories to workspace context and can expand context-file discovery when context.loadFromIncludeDirectories is enabled."
    example: 'qwen -p "query" --include-directories src,docs'
    notes: "Does not directly change system-prompt text, but can cause more QWEN.md context to be loaded."
  - flag: "--all-files / -a"
    mode: modify
    value_shape: "boolean switch"
    description: "Includes all files in the current directory as context."
    example: 'qwen -p "summarize this repo" --all-files'
    notes: "Prompt-adjacent only: it materially changes the model context, not the system instruction layer."
  - flag: "--continue / --resume <ID>"
    mode: other
    value_shape: "boolean switch or session id"
    description: "Resumes a previous session."
    example: 'qwen --continue -p "continue the review" --append-system-prompt "Report only blockers."'
    notes: "Prompt flags are per invocation; re-supply wrapper append/replace flags on resumed runs."
  - flag: "QWEN_WRITE_SYSTEM_MD=<PATH|1|true> qwen ..."
    mode: inspect
    value_shape: "environment variable used as a command prefix"
    description: "Implementation-supported export path that writes the rendered base system prompt to a file."
    example: 'QWEN_WRITE_SYSTEM_MD=/tmp/qwen-system.md qwen -p "noop"'
    notes: "Source code writes the base prompt to the specified path, or to .qwen/system.md when set to 1/true. This is not prominent in user docs. If QWEN_SYSTEM_MD is also set, the exported prompt is the replacement file content, not the built-in prompt."
config_sources:
  - os: macos
    scope: user
    path: "~/.qwen/QWEN.md"
    mode: append
    format: markdown
    notes: "Global context file. Loaded before project/ancestor context; filename is configurable via context.fileName."
  - os: linux
    scope: user
    path: "~/.qwen/QWEN.md"
    mode: append
    format: markdown
    notes: "Global context file. Loaded before project/ancestor context; filename is configurable via context.fileName."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\QWEN.md"
    mode: append
    format: markdown
    notes: "Global context file. Loaded before project/ancestor context; filename is configurable via context.fileName."
  - os: macos
    scope: repo
    path: "QWEN.md"
    mode: append
    format: markdown
    notes: "Project/ancestor context file. Qwen walks from the current working directory upward to the git root or home directory."
  - os: linux
    scope: repo
    path: "QWEN.md"
    mode: append
    format: markdown
    notes: "Project/ancestor context file. Qwen walks from the current working directory upward to the git root or home directory."
  - os: windows
    scope: repo
    path: "QWEN.md"
    mode: append
    format: markdown
    notes: "Project/ancestor context file. Qwen walks from the current working directory upward to the git root or home directory."
  - os: macos
    scope: repo
    path: ".qwen/system.md"
    mode: replace
    format: markdown
    notes: "Implementation-supported full base-prompt replacement when QWEN_SYSTEM_MD is set to 1/true. Not the same as QWEN.md; missing file is fatal."
  - os: linux
    scope: repo
    path: ".qwen/system.md"
    mode: replace
    format: markdown
    notes: "Implementation-supported full base-prompt replacement when QWEN_SYSTEM_MD is set to 1/true. Not the same as QWEN.md; missing file is fatal."
  - os: windows
    scope: repo
    path: ".qwen\\system.md"
    mode: replace
    format: markdown
    notes: "Implementation-supported full base-prompt replacement when QWEN_SYSTEM_MD is set to 1/true. Not the same as QWEN.md; missing file is fatal."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    mode: modify
    format: json
    notes: "Lowest-precedence persisted settings layer. Path can be overridden with QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    mode: modify
    format: json
    notes: "Lowest-precedence persisted settings layer. Path can be overridden with QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    mode: modify
    format: json
    notes: "Lowest-precedence persisted settings layer. Path can be overridden with QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: user
    path: "~/.qwen/settings.json"
    mode: modify
    format: json
    notes: "User settings. Relevant keys include context.fileName, context.includeDirectories, context.loadFromIncludeDirectories, memory.*, telemetry.*, hooks, extensions, skills, and MCP."
  - os: linux
    scope: user
    path: "~/.qwen/settings.json"
    mode: modify
    format: json
    notes: "User settings. Relevant keys include context.fileName, context.includeDirectories, context.loadFromIncludeDirectories, memory.*, telemetry.*, hooks, extensions, skills, and MCP."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\settings.json"
    mode: modify
    format: json
    notes: "User settings. Relevant keys include context.fileName, context.includeDirectories, context.loadFromIncludeDirectories, memory.*, telemetry.*, hooks, extensions, skills, and MCP."
  - os: macos
    scope: repo
    path: ".qwen/settings.json"
    mode: modify
    format: json
    notes: "Project settings. Can set prompt-adjacent context and memory behavior; overrides user settings."
  - os: linux
    scope: repo
    path: ".qwen/settings.json"
    mode: modify
    format: json
    notes: "Project settings. Can set prompt-adjacent context and memory behavior; overrides user settings."
  - os: windows
    scope: repo
    path: ".qwen\\settings.json"
    mode: modify
    format: json
    notes: "Project settings. Can set prompt-adjacent context and memory behavior; overrides user settings."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/settings.json"
    mode: modify
    format: json
    notes: "Highest-precedence persisted settings layer. Path can be overridden with QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: linux
    scope: system
    path: "/etc/qwen-code/settings.json"
    mode: modify
    format: json
    notes: "Highest-precedence persisted settings layer. Path can be overridden with QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    mode: modify
    format: json
    notes: "Highest-precedence persisted settings layer. Path can be overridden with QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: macos
    scope: agent
    path: ".qwen/agents/*.md"
    mode: replace
    format: markdown
    notes: "Project subagent definitions, highest precedence. YAML frontmatter is metadata; Markdown body is the subagent system prompt."
  - os: linux
    scope: agent
    path: ".qwen/agents/*.md"
    mode: replace
    format: markdown
    notes: "Project subagent definitions, highest precedence. YAML frontmatter is metadata; Markdown body is the subagent system prompt."
  - os: windows
    scope: agent
    path: ".qwen\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "Project subagent definitions, highest precedence. YAML frontmatter is metadata; Markdown body is the subagent system prompt."
  - os: macos
    scope: agent
    path: "~/.qwen/agents/*.md"
    mode: replace
    format: markdown
    notes: "User subagent definitions, lower precedence than project agents."
  - os: linux
    scope: agent
    path: "~/.qwen/agents/*.md"
    mode: replace
    format: markdown
    notes: "User subagent definitions, lower precedence than project agents."
  - os: windows
    scope: agent
    path: "%USERPROFILE%\\.qwen\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "User subagent definitions, lower precedence than project agents."
  - os: macos
    scope: extension
    path: "<extension-dir>/QWEN.md"
    mode: append
    format: markdown
    notes: "Extension context may append to QWEN.md context when the extension is installed/enabled."
  - os: linux
    scope: extension
    path: "<extension-dir>/QWEN.md"
    mode: append
    format: markdown
    notes: "Extension context may append to QWEN.md context when the extension is installed/enabled."
  - os: windows
    scope: extension
    path: "<extension-dir>\\QWEN.md"
    mode: append
    format: markdown
    notes: "Extension context may append to QWEN.md context when the extension is installed/enabled."
  - os: macos
    scope: extension
    path: "<extension-dir>/agents/*.md"
    mode: replace
    format: markdown
    notes: "Extension subagents are discovered when the extension is enabled and are read-only through the manager UI."
  - os: linux
    scope: extension
    path: "<extension-dir>/agents/*.md"
    mode: replace
    format: markdown
    notes: "Extension subagents are discovered when the extension is enabled and are read-only through the manager UI."
  - os: windows
    scope: extension
    path: "<extension-dir>\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "Extension subagents are discovered when the extension is enabled and are read-only through the manager UI."
env_vars:
  - name: "QWEN_SYSTEM_MD"
    effect: "Implementation-supported base system prompt replacement. Set to 1/true to read .qwen/system.md, set to a path to read that file, or set to 0/false to disable. Missing replacement file throws."
    mode: replace
  - name: "QWEN_WRITE_SYSTEM_MD"
    effect: "Implementation-supported prompt export. Set to a path to write the rendered base system prompt there, or 1/true to write .qwen/system.md. Avoid combining with QWEN_SYSTEM_MD when exporting the built-in prompt."
    mode: inspect
  - name: "QWEN_HOME"
    effect: "Overrides the global configuration directory, default ~/.qwen. Affects user settings, credentials, memory, skills, and other global state; project .qwen directories are unaffected."
    mode: modify
  - name: "QWEN_RUNTIME_DIR"
    effect: "Overrides runtime output directory for conversations, logs, todos, and related runtime state. Defaults to QWEN_HOME."
    mode: modify
  - name: "QWEN_CODE_SYSTEM_DEFAULTS_PATH"
    effect: "Overrides the system defaults settings JSON path."
    mode: modify
  - name: "QWEN_CODE_SYSTEM_SETTINGS_PATH"
    effect: "Overrides the system settings JSON path."
    mode: modify
  - name: "QWEN_CODE_SAFE_MODE"
    effect: "Equivalent to --safe-mode; disables prompt/context customizations and other custom behavior, but not explicit CLI prompt flags."
    mode: disable
  - name: "QWEN_TELEMETRY_LOG_PROMPTS"
    effect: "Enables or disables telemetry logging of user prompts."
    mode: other
  - name: "QWEN_TELEMETRY_INCLUDE_SENSITIVE_SPAN_ATTRIBUTES"
    effect: "When true/1, attaches verbatim user prompts, system prompts, tool I/O, and model responses to native OpenTelemetry span attributes."
    mode: inspect
prompt_layers:
  - source: "built-in main-session system prompt"
    mode: replace
    scope: ["builtin"]
    order_notes: "Foundational layer unless replaced by --system-prompt or QWEN_SYSTEM_MD."
    notes: "Open source in packages/core/src/core/prompts.ts; not printed by /context or /memory. The prompt can be exported with QWEN_WRITE_SYSTEM_MD."
  - source: "QWEN_SYSTEM_MD replacement file"
    mode: replace
    scope: ["repo", "other"]
    order_notes: "Replaces the built-in base prompt before user memory/context suffixes are appended."
    notes: "Implementation surface rather than prominent user documentation. true/1 reads .qwen/system.md; a path reads that path."
  - source: "--system-prompt"
    mode: replace
    scope: ["session"]
    order_notes: "Runtime CLI override for the main session. User memory/context and --append-system-prompt still follow it."
    notes: "Best wrapper replacement path for typical prompt sizes because it is native and non-mutating."
  - source: "QWEN.md hierarchical context"
    mode: append
    scope: ["user", "repo"]
    order_notes: "Loaded after the built-in or replacement main prompt. Global file first, then current directory/ancestors to git root or home."
    notes: "Filename is configurable via context.fileName. Files can import Markdown via @path/to/file.md. The CLI footer and /memory dialog show loaded context files."
  - source: "managed memory"
    mode: append
    scope: ["user", "repo"]
    order_notes: "Appended as user memory/context when memory features are enabled and data exists."
    notes: "Recent releases lazy-load memory prompt when indexes are empty. /remember is decoupled from QWEN.md and writes to managed memory."
  - source: "extension context"
    mode: append
    scope: ["extension"]
    order_notes: "Loaded when an enabled extension contributes context."
    notes: "Disable extensions with -e none or safe mode for a deterministic baseline."
  - source: "--append-system-prompt"
    mode: append
    scope: ["session"]
    order_notes: "Applied after the base/custom prompt and after loaded memory/context."
    notes: "Best wrapper append path for typical prompt sizes; inline text only."
  - source: "git status reminder"
    mode: append
    scope: ["repo"]
    order_notes: "Source code appends cached git status after the main prompt text when available."
    notes: "Prompt-adjacent implementation detail; not a user-controlled system prompt switch."
  - source: "named subagent system prompt"
    mode: replace
    scope: ["agent", "subagent"]
    order_notes: "When the Agent tool invokes a named subagent, the subagent Markdown body is its own system prompt."
    notes: "Subagents start with separate context. Fork subagents are different: they inherit the parent's exact prompt/history/tools."
agent_prompting:
  supported: true
  definition_surface: "Markdown files with YAML frontmatter in `.qwen/agents/`, `~/.qwen/agents/`, or extension `agents/` directories."
  inheritance: "Named subagents use their own configured prompt and fresh context. Fork subagents selected with subagent_type=\"fork\" inherit the parent's exact system prompt, tools, and conversation history for prompt-cache sharing."
  isolation: "Named subagents maintain separate conversation history and return a final result. Forks run in the background and currently do not feed their result back into the main conversation."
  limitations: "Soft warnings are shown for subagent system prompts over 10,000 characters. CC-compatible fields effort, skills, initialPrompt, memory, and isolation are documented as future follow-up work. Per-agent hooks currently fire globally while the agent runs."
claudine_delivery:
  append_strategy: inline_flag
  replace_strategy: inline_flag
  temp_file_required: false
  argv_limit: "Native Qwen CLI prompt flags accept inline text only; long prompts are subject to OS argv limits. Windows is the tightest practical target. Use temporary files only for fallback strategies."
  notes: "Use --append-system-prompt for append and --system-prompt for normal replacement so user config is not mutated. For very large replacement prompts, QWEN_SYSTEM_MD=<temp-file> is a file-backed fallback but it replaces the built-in base before memory/context and requires controlling env carefully. For very large append prompts, prefer a temporary QWEN_HOME/context-file strategy or reject as too large; Qwen has no native append-file flag. Use --safe-mode and -e none when local discovery must be suppressed."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Qwen Code's documented and native prompt surfaces are Markdown-centric: QWEN.md context, subagent bodies, skills, and .qwen/system.md are all Markdown/text files. Pure Markdown with headings and bullets matches provider conventions. XML-wrapped Markdown, YAML, and JSON are not documented as better system-prompt formats for Qwen CLI and add overhead unless the wrapper itself needs delimiters."
recent_changes:
  - date: "2026-07-03"
    version: "0.19.6"
    change: "Added configurable nested subagents up to a depth limit and web-shell nested subagent tree display."
    impact: "Subagent prompting is no longer strictly one-level in current source, but nested/fork behavior still requires care; wrappers should not assume every child prompt returns directly to the top-level orchestrator."
  - date: "2026-07-03"
    version: "0.19.6"
    change: "Added sessionless workspace memory forget and dream."
    impact: "Memory can be manipulated outside a live session, increasing the importance of safe-mode or shadow QWEN_HOME when wrappers need deterministic prompt layers."
  - date: "2026-07-02"
    version: "0.19.5"
    change: "Lazy-load memory prompt when indexes are empty."
    impact: "Empty managed-memory stores no longer add prompt overhead."
  - date: "2026-07-01"
    version: "0.19.4"
    change: "Added --safe-mode and QWEN_CODE_SAFE_MODE for troubleshooting."
    impact: "Wrappers can suppress most discovered customization layers without touching user config."
  - date: "2026-06-28"
    version: "0.19.3"
    change: "Decoupled /remember from QWEN.md and added git-shared team memory."
    impact: "Persistent instructions are no longer only QWEN.md; managed memory is a separate prompt layer."
quirks:
  - "--system-prompt is a built-in prompt replacement, not a whole-context replacement: QWEN.md and memory are still appended afterward."
  - "--append-system-prompt runs after loaded memory/context; there is no documented way to insert wrapper text before QWEN.md while preserving the built-in prompt."
  - "No native --system-prompt-file or --append-system-prompt-file flag is documented or visible in the current option parser."
  - "QWEN_SYSTEM_MD is implemented in source but is not highlighted in user docs. It gives file-backed replacement, but missing files are fatal and true/1 means .qwen/system.md."
  - "QWEN_WRITE_SYSTEM_MD can export the rendered base prompt to a file. It is useful for research/inspection but writes to disk and should not be used in normal wrapped runs."
  - "If QWEN_SYSTEM_MD and QWEN_WRITE_SYSTEM_MD are both set, the write path captures the replacement prompt, not the built-in prompt."
  - "Qwen's open source prompt includes the built-in text in source; runtime /context and /memory are not full effective-prompt export commands."
  - "context.fileName can point Qwen at AGENTS.md or other filenames, but default discovery is QWEN.md."
  - "Fork subagents inherit the parent prompt/history/tools and currently do not feed results back into the main conversation."
  - "Installed local qwen on this host is Homebrew 0.15.6, while npm reports latest 0.19.6. Local --help behavior should not be treated as current behavior."
gaps:
  - "Did not execute current 0.19.6 from npm because the installed local qwen binary is Homebrew 0.15.6."
  - "No local user settings.json, QWEN.md, or agents were present under /Users/ken/.claudine/.qwen during inspection; only debug files, installation_id, output-language.md, and a skills directory were observed."
  - "Official user docs do not document QWEN_SYSTEM_MD or QWEN_WRITE_SYSTEM_MD; evidence is from source and tests."
  - "No supported single command was found that prints the complete effective prompt including built-in prompt, memory/context, append flag, git status, tool definitions, and runtime reminders."
  - "Exact ordering of extension context relative to all memory sublayers is not fully documented in prose; source confirms these are prompt-affecting layers, but not every merge edge is public."
changes:
  - "Refreshed against Qwen Code 0.19.6 release notes, official docs, and upstream source."
  - "Added QWEN_SYSTEM_MD replacement and QWEN_WRITE_SYSTEM_MD export behavior."
  - "Updated local inspection note for this host's /Users/ken/.claudine/.qwen state and installed qwen 0.15.6 binary."
  - "Updated subagent notes for nested subagents in 0.19.6 and fork inheritance limitations."
requires_claudine_update: false
reason: "Claudine can continue using Qwen's native inline --append-system-prompt and --system-prompt flags. The only implementation consideration is an optional large-prompt fallback using QWEN_SYSTEM_MD for replacement or shadow QWEN_HOME/context files for append; no schema change is required."
---

# Qwen CLI System Prompt Handling

## Overview

Qwen Code has first-class, per-run system prompt controls for the main session:
`--system-prompt` replaces the built-in main-session prompt and
`--append-system-prompt` appends extra instructions. Both are inline-text flags,
and the official docs explicitly allow them to be combined. The important wrapper
detail is that replacement is not a complete context wipe: loaded memory and
context files such as `QWEN.md` are still appended after `--system-prompt`.

The implementation also has two environment-variable prompt surfaces that are
not prominent in user docs. `QWEN_SYSTEM_MD` replaces the base prompt from a file
(`.qwen/system.md` for `1`/`true`, or a custom path), and
`QWEN_WRITE_SYSTEM_MD` writes the rendered base prompt to a file. These are useful
for research and large replacement fallback, but Claudine should prefer the
documented CLI flags for normal wrapper delivery because they are session-scoped
and do not require config mutation.

The local config inspection for this session found Qwen's default user config
root at `/Users/ken/.claudine/.qwen`. It contained debug files,
`installation_id`, `output-language.md`, and a `skills` directory, but no
`settings.json`, `QWEN.md`, or user agent definitions at the inspected depth.
The installed `qwen` binary is Homebrew `0.15.6`; current upstream and npm latest
are `0.19.6`, so local CLI help was not used as current behavioral proof.

## CLI Parameters

| Switch or command | Mode | Wrapper relevance |
| --- | --- | --- |
| `--system-prompt <TEXT>` | Replace | Native, non-mutating replacement for the built-in main-session prompt. Context files and memory still append afterward. |
| `--append-system-prompt <TEXT>` | Append | Native, non-mutating additive instruction layer after built-in/custom prompt and loaded memory/context. |
| `-p, --prompt <TEXT>` | Other | Headless user prompt. Most wrapper runs pair this with prompt flags. |
| `-i, --prompt-interactive <TEXT>` | Other | Interactive session with initial user input. Prompt flags are globally parsed, but official system-prompt examples are headless. |
| `--safe-mode` | Disable | Suppresses context files, hooks, extensions, skills, MCP servers, custom subagents, memory features, and related settings. Does not suppress explicit prompt flags. |
| `-e, --extension <LIST>` | Modify | Use `-e none` to avoid extension-sourced context, skills, MCP, and subagents. |
| `--include-directories <PATHS>` | Modify | Can widen context-file discovery when paired with `context.loadFromIncludeDirectories`. |
| `--all-files, -a` | Modify | Adds file contents to context; not a system-prompt control but changes what the model sees. |
| `--continue`, `--resume <ID>` | Other | Resume runs should re-supply wrapper prompt flags because prompt flags are invocation-scoped. |
| `QWEN_WRITE_SYSTEM_MD=<PATH|1|true> qwen ...` | Inspect | Source-supported export/write of the rendered base system prompt. Use only for inspection, not normal delivery. |

Prompt assembly in the main session source is straightforward:

```mermaid
flowchart TD
  A[Built-in base prompt] --> B{QWEN_SYSTEM_MD?}
  B -- yes --> C[File replacement prompt]
  B -- no --> A
  C --> D{--system-prompt?}
  A --> D
  D -- yes --> E[Inline replacement prompt]
  D -- no --> F[Base prompt]
  E --> G[QWEN.md / memory context]
  F --> G
  G --> H[--append-system-prompt]
  H --> I[git status / runtime reminders]
  I --> J[User prompt]
```

## Configuration and Discovery

Qwen Code settings are JSON and are applied in this documented order:
defaults, system defaults, user settings, project settings, system settings,
environment variables, then command-line arguments. Prompt-related persistent
settings are mostly indirect: they change context filename discovery, memory,
extensions, skills, hooks, and telemetry rather than setting a named
`systemPrompt` key in settings.

| Source | Effect |
| --- | --- |
| `~/.qwen/settings.json` | User-level JSON settings. `QWEN_HOME` can relocate this root. |
| `.qwen/settings.json` | Project-level JSON settings. Overrides user settings. |
| System defaults/settings JSON | Enterprise/system layers; paths differ by OS and can be relocated with documented env vars. |
| `~/.qwen/QWEN.md` | Global Markdown context appended into the system prompt. |
| `QWEN.md` in cwd/ancestors | Project and nested Markdown context appended into the system prompt. |
| `.qwen/system.md` | Full built-in prompt replacement only when `QWEN_SYSTEM_MD=1` or `true`. |
| `.qwen/agents/*.md`, `~/.qwen/agents/*.md` | Subagent definitions; Markdown body is the subagent system prompt. |
| Extension context/agents | Loaded only when the extension is enabled. |

`context.fileName` can be a string or array of strings, so Qwen can be made to
load `AGENTS.md` or another filename. The default is Qwen's own context file
name, `QWEN.md`. Context files can import other Markdown files with
`@path/to/file.md`. The `/memory` dialog shows loaded context files and their
paths; `/context` reports context usage and has a detail mode, but neither is a
complete built-in prompt export.

## Prompt Layers and Precedence

The wrapper-critical precedence rules are:

1. The built-in prompt is the default base.
2. `QWEN_SYSTEM_MD` can replace that base from a file before normal prompt
   suffixes are added.
3. `--system-prompt` replaces the main-session base prompt for the invocation.
4. Loaded memory and context files, including `QWEN.md`, append after the base
   or replacement.
5. `--append-system-prompt` appends after loaded memory/context.
6. Subagents have separate prompt rules: named subagents use their own Markdown
   body; forks inherit the parent's exact system prompt/history/tools.

`--safe-mode` is the practical diagnostic switch. It removes most discovery
layers but leaves explicit prompt flags active, which makes it useful for
checking Claudine delivery without persistent local customization noise.

## Agents and Subagents

Qwen Code supports user-defined subagents. Agent files are Markdown with YAML
frontmatter:

```markdown
---
name: rigorous-reviewer
description: Deep code review with a turn cap
approvalMode: plan
tools:
  - read_file
  - grep_search
---

You are a code reviewer. Analyze the code thoroughly and report findings
ordered by severity.
```

The body is the subagent system prompt. Frontmatter controls metadata such as
`name`, `description`, `model`, `approvalMode`, `tools`, `disallowedTools`,
`permissionMode`, `maxTurns`, `color`, `mcpServers`, and `hooks`. Project agents
in `.qwen/agents/` have higher precedence than user agents in
`~/.qwen/agents/`; extension agents are discovered when the extension is
enabled.

Named subagents start with separate context and return a final result. Fork
subagents are selected explicitly with `subagent_type: "fork"` and inherit the
parent's exact API request prefix for prompt-cache sharing. Current docs note
that fork results are visible in UI progress but are not automatically fed back
into the main conversation. Qwen Code `0.19.6` added configurable nested
subagent spawning and web-shell tree display, so wrappers should avoid assuming
a flat one-parent/one-child prompt topology.

## Format Recommendations

Use pure Markdown for both append and replacement prompts.

| Goal | Recommended format | Reason |
| --- | --- | --- |
| Append | Markdown | Matches Qwen's context-file and subagent conventions. Headings, lists, and short paragraphs are natural in `QWEN.md`-style prompt layers. |
| Replace | Markdown | The full prompt is text; Qwen's own `.qwen/system.md`, `QWEN.md`, skills, and subagent bodies are Markdown/text-oriented. |

XML-wrapped Markdown is not documented as beneficial for Qwen CLI. YAML and JSON
are useful for subagent frontmatter or machine data, not as the primary system
prompt format. For replacements, include any safety, tool-use, and operating
context that the built-in prompt would otherwise provide.

## Recent Changes

| Date | Version | Change | Prompt impact |
| --- | --- | --- | --- |
| 2026-07-03 | 0.19.6 | Added nested subagents up to configurable depth; web-shell displays nested subagents as a tree. | Subagent prompt topology can be nested; wrappers should not assume one-level delegation. |
| 2026-07-03 | 0.19.6 | Added sessionless workspace memory forget and dream. | Memory layers can be modified outside a live session. |
| 2026-07-02 | 0.19.5 | Lazy-load memory prompt when indexes are empty. | Empty memory stores no longer add prompt overhead. |
| 2026-07-01 | 0.19.4 | Added `--safe-mode` and `QWEN_CODE_SAFE_MODE`. | Provides a non-mutating way to suppress most discovered customization layers. |
| 2026-06-28 | 0.19.3 | `/remember` stopped writing to `QWEN.md`; team memory tier added. | Persistent instructions are split between context files and managed memory. |

## Quirks and Workarounds

- `--system-prompt` replaces the built-in prompt only. It does not remove
  `QWEN.md`, managed memory, extension context, or appended instructions.
- `--append-system-prompt` is ordered after loaded memory/context; there is no
  native flag to append before `QWEN.md`.
- Qwen has no documented prompt-file variants for the native prompt flags.
  Wrapper prompts must be inline unless using an env/config fallback.
- `QWEN_SYSTEM_MD` is a real source-supported replacement mechanism, but it is
  an environment/file mechanism with fatal missing-file behavior and should be
  treated as a fallback rather than the default wrapper path.
- `QWEN_WRITE_SYSTEM_MD` can inspect/export the base prompt, but it writes a
  file and can overwrite `.qwen/system.md` when set to `1`/`true`.
- Combining `QWEN_SYSTEM_MD` and `QWEN_WRITE_SYSTEM_MD` exports the replacement
  prompt, not the built-in prompt.
- `QWEN_TELEMETRY_INCLUDE_SENSITIVE_SPAN_ATTRIBUTES=true` can send verbatim
  system prompts to telemetry backends. This is an inspection surface, but it is
  not suitable as a safe wrapper export mechanism.
- Fork subagents inherit parent context for prompt-cache sharing and currently
  do not feed their final result into the parent conversation.
- Per-agent hooks are documented as v1-limited: while a subagent runs, its hook
  entries fire for every matching event in the session, not only that
  subagent's own tool calls.

## Claudine Delivery Notes

Claudine should use:

- Append: `qwen --append-system-prompt <prepared-markdown> ...`
- Replace: `qwen --system-prompt <prepared-markdown> ...`

This is native, non-mutating, and aligns with Qwen's documented CLI contract. For
normal prompt sizes, no temporary file is required. Because both flags are inline
only, Claudine should keep argv limits in mind, especially on Windows. If a
prompt is too large:

- Replacement fallback: write a temporary Markdown file and launch with
  `QWEN_SYSTEM_MD=<temp-file>`, while also controlling `QWEN_HOME`/cwd and
  cleanup. This is file-backed but less officially documented than the native
  flag.
- Append fallback: there is no equivalent append-file flag. Use a temporary
  shadow `QWEN_HOME` and/or launch directory with a controlled `QWEN.md` only if
  preserving built-in prompt plus file-backed appended context is more important
  than strict `--append-system-prompt` semantics.
- Deterministic runs: add `--safe-mode` and `-e none` when local memory,
  extension, subagent, skill, hook, and MCP discovery must be suppressed.

Do not mutate the user's `~/.qwen/settings.json`, `~/.qwen/QWEN.md`,
project `QWEN.md`, or project `.qwen/system.md` for wrapper prompt delivery.

## Changelog

- 2026-07-03: Refreshed from Qwen Code `0.19.6` official docs, source, and
  changelog. Added `QWEN_SYSTEM_MD`, `QWEN_WRITE_SYSTEM_MD`, current subagent
  nesting behavior, and local config observations.

## Sources

- [Qwen Code configuration settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code headless mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code subagents](https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents/)
- [Qwen Code commands](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [QwenLM/qwen-code repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code changelog](https://github.com/QwenLM/qwen-code/blob/main/CHANGELOG.md)
- [Qwen Code prompt source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/core/prompts.ts)
- [Qwen Code CLI option parser](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/config/config.ts)
- [Qwen Code main-session prompt assembly](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/core/client.ts)
