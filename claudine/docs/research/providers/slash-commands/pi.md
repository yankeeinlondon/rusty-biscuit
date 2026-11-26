---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://pi.dev/
docs: https://pi.dev/docs/latest
slash_docs: https://pi.dev/docs/latest/prompt-templates
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.pi/agent/prompts/<name>.md
    notes: Global prompt templates. Loaded in every project. Pi resolves ~ via the user's home directory.
  - os: linux
    scope: user
    path: ~/.pi/agent/prompts/<name>.md
    notes: Same behavior as macOS.
  - os: windows
    scope: user
    path: ~/.pi/agent/prompts/<name>.md
    notes: Same ~ resolution as macOS/Linux through Node.js os.homedir(). No documented separate %APPDATA% path.
  - os: macos
    scope: repo
    path: .pi/prompts/<name>.md
    notes: Project prompt templates. Loaded only after the project is trusted.
  - os: linux
    scope: repo
    path: .pi/prompts/<name>.md
    notes: Same behavior as macOS.
  - os: windows
    scope: repo
    path: .pi/prompts/<name>.md
    notes: Same trust-gated behavior as macOS/Linux.
  - os: macos
    scope: extension
    path: ~/.pi/agent/extensions/*.ts or .pi/extensions/*.ts
    notes: TypeScript extensions can register arbitrary / commands at runtime. Project extensions require trust.
  - os: linux
    scope: extension
    path: ~/.pi/agent/extensions/*.ts or .pi/extensions/*.ts
    notes: Same behavior as macOS.
  - os: windows
    scope: extension
    path: ~/.pi/agent/extensions/*.ts or .pi/extensions/*.ts
    notes: Same behavior as macOS/Linux.
  - os: macos
    scope: user
    path: ~/.pi/agent/skills/ or ~/.agents/skills/
    notes: Skills loaded on demand and invoked as /skill:name. Also applies to Linux and Windows.
  - os: macos
    scope: repo
    path: .pi/skills/ or .agents/skills/
    notes: Project skills. Loaded only after trust. Also applies to Linux and Windows.
format:
  file_names:
    - "*.md"
  frontmatter: true
  required_fields: []
  optional_fields:
    - description
    - argument-hint
  argument_syntax: |
    $1, $2, ... for positional arguments (1-indexed).
    $@ or $ARGUMENTS for all arguments joined.
    ${1:-default} for arg 1 with a default fallback.
    ${@:N} for arguments from the Nth position (1-indexed).
    ${@:N:L} for L arguments starting at N.
  body_format: markdown
  notes: |
    Prompt templates are plain Markdown files. The filename (without .md) becomes the / command name.
    The body is expanded into the user prompt with shell-style argument substitution.
    The only recognized frontmatter fields are description and argument-hint.
    Multi-word arguments are grouped with double quotes: /component Button "click handler".
    Discovery in prompts/ directories is non-recursive.
command_model:
  invocation: |
    Type /name in the editor, where name is the prompt template filename without .md.
    Example: /review expands review.md; /component Button "click handler" passes arguments.
    Skills are invoked as /skill:name with optional arguments.
    Extension commands are invoked as /name and are checked before skills and templates.
  namespacing: |
    Built-in slash commands, extension commands, skill commands, and prompt templates share the / namespace.
    Input processing order: extension commands first, then /skill:name expansion, then prompt-template expansion, then agent processing.
    Name collisions between prompt templates in different scopes are resolved by Pi's resource loading order; the docs do not define explicit precedence rules between user and project templates.
    Skill names are governed by the Agent Skills specification and may differ from the parent directory; Pi warns on collisions and keeps the first skill found.
  arguments: |
    Arguments are split by whitespace. Double quotes group multi-word values into one argument.
    Inside the template body, $1, $2, ... substitute positional arguments; $@ and $ARGUMENTS substitute the whole argument string.
    ${1:-default} provides a default. ${@:N} and ${@:N:L} slice the argument list.
    For skills, all arguments after /skill:name are appended to the skill content as User: <args>.
    For extension commands, the handler receives the raw argument string and parses it itself.
  output_handling: |
    Prompt templates: the rendered Markdown body is inserted into the conversation as a user prompt.
    Skills: the SKILL.md content is read on demand and added to context, with User: <args> appended if arguments were supplied.
    Extension commands: the registered TypeScript handler decides what to do; it may send messages, run tools, update UI, or inject prompts.
  disabled_mechanism: |
    Remove or rename the template/skill file or extension source.
    Pass --no-prompt-templates to disable all prompt-template discovery.
    Pass --no-skills to disable skill discovery (explicit --skill paths still load).
    Pass --no-extensions to disable extension discovery (explicit -e paths still load).
    Use pi config to enable/disable resources from installed packages.
  notes: |
    Project-local prompts, skills, and extensions require project trust before they load.
    Non-interactive modes (-p, --mode json, --mode rpc) use defaultProjectTrust from settings; pass --approve or --no-approve to override.
    Use /reload to re-scan extensions, skills, prompts, and context files in a running session.
    Prompt template discovery is non-recursive; subdirectories must be added via settings or package manifests.
portability:
  portable: false
  non_portable_assets:
    - "$1 / $2 / $@ / $ARGUMENTS shell-style placeholders"
    - "${1:-default}, ${@:N}, ${@:N:L} shell parameter expansions"
    - "/skill:name namespace prefix and skill loading mechanics"
    - "TypeScript extension command handlers"
    - "Pi-specific frontmatter (description, argument-hint only; no tool/model overrides)"
  rewrite_needed: true
  notes: |
    The Markdown prose body of a prompt template is largely portable, but the argument substitution syntax must be rewritten for providers that use different placeholders (e.g., $ARGUMENTS, {{args}}, $0/$1).
    Skills implement the Agent Skills standard, so their SKILL.md structure is more portable than Pi-specific prompt templates, but invocation as /skill:name and allowed-tools semantics are provider-specific.
    Extension commands are TypeScript modules tied to Pi's ExtensionAPI and are not portable.
cli_params:
  - flag: --no-prompt-templates, -np
    description: Disable prompt template discovery and loading.
    example: pi --no-prompt-templates
  - flag: --prompt-template <path>
    description: Load a prompt template file or directory; repeatable.
    example: pi --prompt-template ./prompts
  - flag: --no-skills, -ns
    description: Disable skill discovery and loading.
    example: pi --no-skills
  - flag: --skill <path>
    description: Load a skill file or directory; repeatable, additive even with --no-skills.
    example: pi --skill ./skills
  - flag: --no-extensions, -ne
    description: Disable extension discovery; explicit -e paths still load.
    example: pi --no-extensions
  - flag: --extension <path>, -e <path>
    description: Load an extension file or directory; repeatable.
    example: pi -e ./my-extension.ts
  - flag: --no-context-files, -nc
    description: Disable AGENTS.md and CLAUDE.md discovery.
    example: pi --no-context-files
  - flag: --approve, -a
    description: Trust project-local files for this run.
    example: pi --approve
  - flag: --no-approve, -na
    description: Ignore project-local files for this run.
    example: pi --no-approve
  - flag: --no-themes
    description: Disable theme discovery and loading.
    example: pi --no-themes
  - flag: --theme <path>
    description: Load a theme file or directory; repeatable.
    example: pi --theme ./themes
env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: Override the config directory; default is ~/.pi/agent.
  - name: PI_OFFLINE
    effect: Disable startup network operations, including update checks and package update checks.
  - name: PI_SKIP_VERSION_CHECK
    effect: Skip the Pi version update check at startup.
  - name: PI_TELEMETRY
    effect: Override install/update telemetry when set to 1/true/yes or 0/false/no.
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: Override session storage directory; overridden by --session-dir.
  - name: PI_PACKAGE_DIR
    effect: Override package directory, useful for Nix/Guix store paths.
changes: []
requires_claudine_update: true
reason: Pi is a new provider on the research roster with first-class prompt-template and extension-command support. Claudine should add Pi to the slash-command linking model, classify prompt templates as rewrite-needed (argument syntax), and avoid linking TypeScript extension commands.
---

# Pi Slash Commands, Prompt Templates, and Extension Commands

## Overview

Pi calls its primary user-defined reusable command feature **prompt templates**: Markdown files that expand into full prompts when invoked with `/name`. Pi also exposes **skills** as `/skill:name` commands and lets **extensions** register arbitrary `/` commands via TypeScript. Support is **first class** because users can define named, documented, argument-taking commands at user, project, and package scope, and invoke them from the editor.

This document focuses on the command-shaped surfaces: prompt templates (the closest equivalent to slash commands), skill commands, and extension commands. Packaging and discovery of skills themselves are covered by the skills research topic; this topic owns the invocation grammar and command-shaped entries.

## Locations

Pi's config root defaults to `~/.pi/agent/` and can be overridden with `PI_CODING_AGENT_DIR`. Project-local resources live under `.pi/`. The docs do not define OS-specific path differences; Pi resolves `~` through Node.js `os.homedir()` on macOS, Linux, and Windows.

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux / Windows | User | `~/.pi/agent/prompts/<name>.md` | Global prompt templates. Loaded in every project. |
| macOS / Linux / Windows | Repo | `.pi/prompts/<name>.md` | Project prompt templates. Loaded only after the project is trusted. |
| macOS / Linux / Windows | User | `~/.pi/agent/skills/` and `~/.agents/skills/` | Global skills; invoked as `/skill:name`. |
| macOS / Linux / Windows | Repo | `.pi/skills/` and `.agents/skills/` | Project skills; loaded only after trust. |
| macOS / Linux / Windows | Extension | `~/.pi/agent/extensions/*.ts` or `.pi/extensions/*.ts` | TypeScript extensions that can register `/` commands. Project extensions require trust. |
| All | Package | `prompts/`, `skills/`, `extensions/` directories or `pi.prompts` / `pi.skills` / `pi.extensions` entries in `package.json` | Resources from installed Pi packages. |
| All | Settings | `prompts`, `skills`, `extensions` arrays in `~/.pi/agent/settings.json` or `.pi/settings.json` | Additional explicit paths; supports glob patterns and exclusions. |

### Local observations

On this machine, `~/.pi/agent/` exists and contains `settings.json`, `models.json`, `auth.json`, and a `sessions/` directory. `~/.pi/agent/prompts/` and `~/.pi/agent/skills/` do **not** exist. No project-level `.pi/` directories were found in the current workspace.

## File Format

### Prompt templates

A prompt template is a single Markdown file:

```text
~/.pi/agent/prompts/
└── review.md
```

The filename becomes the command name: `review.md` → `/review`.

```markdown
---
description: Review staged git changes
argument-hint: "[files]"
---
Review the staged changes (`git diff --cached`). Focus on:
- Bugs and logic errors
- Security issues
- Error handling gaps
```

| Field | Required | Purpose |
| :---- | :------- | :------ |
| `description` | No | Shown in the `/` autocomplete dropdown. If omitted, the first non-empty line of the body is used. |
| `argument-hint` | No | Hint displayed before the description in autocomplete. Use `<angle brackets>` for required args and `[square brackets]` for optional args. |

The body is Markdown. It is expanded into the user prompt with shell-style argument substitution.

### Argument grammar

| Token | Meaning |
| :---- | :------ |
| `$1`, `$2`, ... | Positional argument (1-indexed). |
| `$@` | All arguments joined. |
| `$ARGUMENTS` | Alias for `$@`; all arguments joined. |
| `${1:-default}` | Arg 1 if present/non-empty, otherwise `default`. |
| `${@:N}` | Arguments from the Nth position onward (1-indexed). |
| `${@:N:L}` | `L` arguments starting at position `N`. |

Example template:

```markdown
---
description: Create a component
---
Create a React component named $1 with features: $@
```

Usage:

```text
/component Button "onClick handler" "disabled support"
```

Resulting prompt:

```text
Create a React component named Button with features: Button "onClick handler" "disabled support"
```

Multi-word arguments must be quoted with double quotes. Default values are supported via `${1:-default}`.

### Skills

Skills follow the [Agent Skills standard](https://agentskills.io/specification). A skill is a directory containing `SKILL.md` with required frontmatter `name` and `description`:

```markdown
---
name: brave-search
description: Web search and content extraction via Brave Search API.
---
```

Pi allows the `name` frontmatter to differ from the parent directory, unlike the strict standard. Skills are loaded on demand and invoked as `/skill:name`.

### Extension commands

Extensions are TypeScript modules loaded via jiti. They register commands programmatically:

```typescript
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  pi.registerCommand("hello", {
    description: "Say hello",
    handler: async (args, ctx) => {
      ctx.ui.notify(`Hello ${args || "world"}!`, "info");
    },
  });
}
```

Extension commands have no declarative file format; they are code.

## Invocation Model

### How commands are invoked

In the interactive editor, type `/` to open autocomplete, then the command name.

```text
/review                           # Expands review.md
/component Button                 # Pass one argument
/component Button "click handler" # Pass multiple arguments
/skill:brave-search "Rust crates" # Load and run a skill
/hello world                      # Extension command
```

### Namespacing and precedence

Built-in slash commands, extension commands, skill commands, and prompt templates all share the `/` namespace. Input is processed in this order:

1. **Extension commands** (`/cmd`) are checked first; if found, the handler runs and skill/template expansion is skipped.
2. **`input` event** handlers can transform or handle input before expansion.
3. **Skill commands** (`/skill:name`) are expanded to skill content.
4. **Prompt templates** (`/name`) are expanded to template content.
5. Agent processing begins.

Name collisions between prompt templates are resolved by Pi's resource loading order, but the docs do not publish explicit precedence rules between user and project templates. For skills, Pi warns on collisions and keeps the first skill discovered.

### Arguments

Prompt templates split arguments by whitespace; double quotes group multi-word values. The body uses shell-style `$1`, `$@`, `${1:-default}`, `${@:N}`, and `${@:N:L}` substitutions.

Skills receive all text after `/skill:name` as a single raw string appended as `User: <args>`.

Extension command handlers receive the raw argument string and parse it themselves.

### Output handling

- **Prompt templates**: the rendered Markdown body becomes a user prompt.
- **Skills**: the `SKILL.md` content is read into context on demand; arguments are appended.
- **Extension commands**: the TypeScript handler controls output, which may include UI notifications, injected messages, tool calls, or prompt modifications.

### Disable mechanisms

- Delete, rename, or move the file/extension.
- `--no-prompt-templates` disables all prompt template discovery.
- `--no-skills` disables skill discovery (explicit `--skill` paths still load).
- `--no-extensions` disables extension discovery (explicit `-e` paths still load).
- `pi config` enables or disables resources from installed packages.

### Trust and permissions

Project-local `.pi/prompts/`, `.pi/skills/`, `.pi/extensions/`, and `.pi/settings.json` load only after the project is trusted. In non-interactive modes (`-p`, `--mode json`, `--mode rpc`), Pi uses the `defaultProjectTrust` setting (`ask`, `always`, `never`). Pass `--approve`/`-a` or `--no-approve`/`-na` to override for one run. Use `/trust` in interactive mode to save a trust decision.

### Live reload

Use `/reload` to re-scan extensions, skills, prompts, and context files in a running session.

## Portability

Pi prompt templates are **not portable** without rewriting.

What can be linked with rewrites:

- The Markdown prose body, after mapping argument placeholders to the target provider's grammar.

What is provider-specific and must be rewritten or removed:

- `$1`, `$2`, `$@`, `$ARGUMENTS`, `${1:-default}`, `${@:N}`, `${@:N:L}` placeholders.
- The `/skill:name` invocation prefix and skill-specific semantics such as `allowed-tools`.
- TypeScript extension command handlers tied to Pi's `ExtensionAPI`.
- The limited `description`/`argument-hint` frontmatter (other providers may support richer metadata).

Skills follow the Agent Skills standard, so their directory structure and `SKILL.md` frontmatter are more portable than prompt templates, but the `/skill:name` invocation surface and Pi's relaxed name rules still require mapping. Extension commands are fully provider-specific.

## Claudine Linking Notes

- Classify **Pi** as **first-class slash-command support** with **non-portable** command files.
- Link **prompt templates** as the primary Pi slash-command equivalent. Map the Markdown body and rewrite shell-style argument placeholders (`$1`, `$@`, `${1:-default}`) to the target provider's argument grammar.
- **Skills** are packaged capabilities; this topic owns their `/skill:name` invocation surface. Claudine can consider linking `SKILL.md` content where the target provider supports the Agent Skills standard, but must map the invocation prefix and tool-permission metadata.
- **Do not link** TypeScript extension command sources to other providers; they depend on Pi's `ExtensionAPI` and runtime.
- Project trust is Pi-specific; other providers may need their own opt-in mechanisms for repo-level commands.
- Pi's command precedence (extension commands → skills → prompt templates) should be preserved if Claudine builds a unified command index.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi documentation](https://pi.dev/docs/latest)
- [Using Pi — slash commands and CLI reference](https://pi.dev/docs/latest/usage)
- [Prompt templates](https://pi.dev/docs/latest/prompt-templates)
- [Skills](https://pi.dev/docs/latest/skills)
- [Extensions — registerCommand and input order](https://pi.dev/docs/latest/extensions)
- [Settings — resources and project trust](https://pi.dev/docs/latest/settings)
- [Pi packages](https://pi.dev/docs/latest/packages)
- [Pi GitHub repository](https://github.com/earendil-works/pi)
- Local `pi --help` output and inspection of `~/.pi/agent/`
