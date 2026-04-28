---
homepage: https://roocode.com/
docs: https://docs.roocode.com/
cli_docs: https://github.com/RooCodeInc/Roo-Code/tree/main/apps/cli
---

# Roo Code CLI

Roo Code is an AI-powered coding assistant that originated as a VS Code extension
(forked from Cline) and now ships a standalone CLI (`roo`) that runs the same
agent outside of VS Code. The CLI uses `@roo-code/vscode-shim` to provide a
VS Code API compatibility layer so the full extension logic runs inside a plain
Node.js process.

The binary is called **`roo`** and lives in the monorepo at `apps/cli/`.

> **Sunset Notice (April 2026):** Roo Code announced that all products
> (Extension, Cloud, Router) will shut down on **May 15, 2026**. The extension
> repository will be archived. A community team is working on an official
> handoff so the extension may continue under community maintenance. See
> [Sunsetting Roo Code](https://docs.roocode.com/sunset) for details.


## Model Specification

Use `--provider` and `--model` to select the LLM.

```bash
roo --provider anthropic --model claude-sonnet-4-20250514 "Explain this repo"
roo --provider openrouter --model anthropic/claude-opus-4.6 "Fix the tests"
roo --provider gemini --model gemini-2.5-pro "Summarize"
```

**Defaults:**

| Parameter    | Default value                                                     |
|--------------|-------------------------------------------------------------------|
| `--provider` | `openrouter` (falls back to `roo` when authenticated with Cloud)  |
| `--model`    | `anthropic/claude-opus-4.6`                                       |

API keys can be supplied via `--api-key` or through the matching environment
variable (see "Environment Variables" below). When neither is provided the CLI
checks the credential store written by `roo auth login`.

In the VS Code extension, models are configured per-mode through **API
Configuration Profiles** (Settings > Providers). Each profile stores the
provider, model, temperature, thinking budget, and rate-limit settings.
**Sticky Models** remember the last-used model per mode across sessions.

**Limitations:**

- The CLI does not support API Configuration Profiles; provider and model must
  be specified on every invocation or defaulted.
- There is no `.rooconfig` file that the CLI reads for default provider/model;
  environment variables are the only persistent alternative to flags.


## Non-interactive Engagement

The CLI supports several non-interactive modes.

### Print Mode (`--print`)

Single-shot execution: send a prompt, get a response, exit.

```bash
roo --print "Summarize this repository"
roo --print --output-format json "List all exported functions"
```

- A prompt is **required** (positional argument or `--prompt-file`).
- Output format is controlled via `--output-format`: `text` (default), `json`,
  or `stream-json`.
- All actions are auto-approved (no interactive prompts).
- Optionally specify a session ID with `--create-with-session-id`:

```bash
roo --print --create-with-session-id 018f7fc8-7c96-7f7c-98aa-2ec4ff7f6d87 "Summarize this repository"
```

### Stdin Stream Mode (`--stdin-prompt-stream`)

Send structured NDJSON commands via stdin for programmatic control of a
long-lived process. Replaced the earlier raw-text-per-line protocol in
v0.1.0 (February 2026).

```bash
printf '{"command":"start","requestId":"1","prompt":"1+1=?"}\n' | roo --print --stdin-prompt-stream --output-format stream-json
```

- Requires `--print` and `--output-format stream-json`.
- NDJSON commands: `start`, `message`, `cancel`, `ping`, `shutdown`.
- Each `start` command accepts an optional `taskId` (UUID) and `images`
  (array of base64 data URIs).
- Lifecycle events: `ack`, `done`, `error` with `requestId` correlation.
- Use `--signal-only-exit` to keep the process alive for harness-based
  orchestration (only exits on SIGINT/SIGTERM).

### Oneshot Mode (`--oneshot`)

Runs interactively but exits automatically upon task completion.

```bash
roo --oneshot "Create a TODO.md"
```

### Ephemeral Mode (`--ephemeral`)

Runs without persisting any state (uses a temporary storage directory).

```bash
roo --ephemeral --print "What version of Node is installed?"
```

### Prompt File (`--prompt-file`)

Read the prompt from a file instead of the command line.

```bash
roo --print --prompt-file instructions.md
```

### Session Resume (`--session-id`, `--continue`)

Resume a previous task by session ID or continue the most recent one:

```bash
roo --session-id 018f7fc8-7c96-7f7c-98aa-2ec4ff7f6d87 "Continue"
roo --continue "Pick up where we left off"
```

- `--session-id <uuid>` loads a specific session.
- `-c, --continue` resumes the most recent task in the current workspace.
- Use `roo list sessions` to discover available session IDs.


## Subscription versus Per Call API

There are two usage models:

1. **Bring Your Own Key (BYOK)** -- Supply an API key for any supported
   provider (Anthropic, OpenAI, OpenRouter, Gemini, etc.) via `--api-key` or
   the matching environment variable. You pay the provider directly per token.
   This works in both the VS Code extension and the CLI.

2. **Roo Code Cloud** -- Authenticate with `roo auth login` and set
   `--provider roo`. Roo Code Cloud offers curated models (Gemini, GPT,
   Claude) with no markup, billed via pre-paid credits (denominated in USD).
   Cloud Agents cost $5/hour in credits while running.

   Pricing tiers:
   - **VS Code Extension**: Free + inference costs (BYOK or credits).
   - **Cloud Free**: $0/month + credits for Cloud Agents and Router.
   - **Cloud Team**: $99/month + credits; unlimited users, shared config,
     centralized billing.
   - **Enterprise**: Custom pricing.

> **Note:** Roo Code Cloud and Router will shut down on May 15, 2026.


## System Prompt

### Custom Instructions (Supplement)

Custom instructions **supplement** the built-in system prompt without replacing
it. They are appended in this order:

1. Language preference
2. Global instructions (Prompts tab UI)
3. Mode-specific instructions (Prompts tab UI)
4. Mode-specific rule directories (`~/.roo/rules-{modeSlug}/` and `.roo/rules-{modeSlug}/`)
5. `.roorules-{modeSlug}` file (fallback)
6. `.rooignore` instructions
7. `AGENTS.md` or `AGENT.md`
8. General rule directories (`~/.roo/rules/` and `.roo/rules/`)
9. `.roorules` file (fallback)

File-based rules are read recursively in alphabetical order by filename.
Workspace rules take precedence over global rules when conflicts arise.

### Footgun Prompting (Replace)

Create `.roo/system-prompt-{mode-slug}` (e.g., `.roo/system-prompt-code`) to
**replace** the standard system prompt for a specific mode. The final prompt
becomes:

1. Core `roleDefinition` (always preserved)
2. Your override file content
3. Any `customInstructions` (preserved)

Standard sections (tool descriptions, rules, capabilities) are bypassed.

Template variables available in the override file: `{{mode}}`, `{{language}}`,
`{{shell}}`, `{{operatingSystem}}`, `{{workspace}}`.

An icon appears in the VS Code chat input when an override is active. Empty
override files are ignored.


## Permissions

### CLI Defaults

The CLI **auto-approves all actions by default** (tool executions, commands,
browser, MCP). Followup questions auto-select the first suggestion after a
60-second timeout.

Use `--require-approval` to restore manual approval prompts for every action.

### VS Code Extension Defaults

All actions require manual approval by default. The auto-approve panel
(toggled with `Cmd+Alt+A` / `Ctrl+Alt+A`) provides granular control over
eight permission categories:

| Category                       | Risk   | Description                                       |
|--------------------------------|--------|---------------------------------------------------|
| Read Files & Directories       | Medium | View directory contents and read files             |
| Edit Files                     | High   | Create and edit files (configurable write delay)   |
| Execute Approved Commands      | High   | Terminal commands via allowlist/denylist            |
| Use Browser                    | Medium | Headless browser interaction                       |
| Use MCP Servers                | Medium | Requires global toggle AND per-tool "Always allow" |
| Switch Modes                   | Low    | Automatic mode changes and creation                |
| Create & Complete Subtasks     | Low    | Boomerang task orchestration                       |
| Answer Follow-Up Questions     | Low    | Auto-select default after timeout (1-300 seconds)  |

There is no single "yolo" toggle. The `All` chip enables all categories at
once, and the `Enabled` master toggle pauses/resumes all auto-approval while
preserving individual selections.


## Thinking Level

Reasoning effort is controlled via `--reasoning-effort` on the CLI:

```bash
roo --reasoning-effort high "Redesign the auth module"
```

Supported values: `unspecified`, `disabled`, `none`, `minimal`, `low`,
`medium` (default), `high`, `xhigh`.

In the VS Code extension, the reasoning/thinking budget is configured
per-profile in the provider settings UI:

- **Anthropic**: Enable "Reasoning Mode" and adjust the thinking budget slider.
- **Gemini**: "Set Reasoning Budget" checkbox exposes a budget slider (minimum
  128 tokens, increased from 1024 in v3.25).
- **OpenAI / OpenRouter**: `reasoningEffort` field (`low`, `medium`, `high`).

Models with thinking capabilities require a fixed temperature of 1.0.


## Logging

### CLI

- Pass `--debug` for detailed debug output including prompts, paths, and
  internal state.
- Debug log file at `~/.roo/cli-debug.log` is disabled by default; enabled
  only when `--debug` is passed.
- `@roo-code/core` includes a file-based debug logging module
  (`debug-log/index.ts`) for structured log output.

### VS Code Extension

- Task history is stored in VS Code's extension global storage:
  - macOS: `~/Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/`
  - Linux: `~/.config/Code/User/globalStorage/rooveterinaryinc.roo-cline/`
- A custom storage path can be configured via the VS Code setting
  `roo-cline.customStoragePath` or the command `roo-cline.setCustomStoragePath`.
- Checkpoints (shadow Git snapshots) are created before file modifications.
- Settings can be exported to JSON via the settings management UI.


## Installation

### Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/install.sh | sh
```

**Requirements:** Node.js 20+, macOS Apple Silicon (M1/M2/M3/M4) or
Linux x64/arm64.

**Custom install directory:**

```bash
ROO_INSTALL_DIR=/opt/roo-code ROO_BIN_DIR=/usr/local/bin curl -fsSL ... | sh
```

**Pin a specific version:**

```bash
ROO_VERSION=0.1.17 curl -fsSL https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/install.sh | sh
```

### Update

```bash
roo upgrade
```

Or re-run the install script.

### Uninstall

```bash
rm -rf ~/.roo/cli ~/.local/bin/roo
```


## CLI Switch Summary

Every CLI switch and positional argument, with description, default, and
examples. Sourced from the CLI entry point (`apps/cli/src/index.ts`) as of
v3.53.0 (CLI v0.1.17).

### Positional Argument

#### `[prompt]`

The initial prompt to send to the agent. Optional -- if omitted the CLI
starts in interactive TUI mode where you can type the prompt.

**Default:** None (interactive TUI mode starts without a prompt).

```bash
roo "What is this project?"
roo -w ~/Documents/my-project
```

### Prompt Input

#### `--prompt-file <path>`

Read the prompt from a file instead of the command-line argument. Useful for
long multi-line prompts or prompts stored alongside code.

**Default:** None.

```bash
roo --print --prompt-file instructions.md
roo --prompt-file prompt.txt -w ~/my-project
```

#### `--create-with-session-id <session-id>`

Create a new task using a specific session ID (must be a valid UUID). Allows
external systems to track and reference sessions by a known identifier.

**Default:** None (auto-generated UUID).

```bash
roo --print --create-with-session-id 018f7fc8-7c96-7f7c-98aa-2ec4ff7f6d87 "Summarize this repository"
```

#### `--session-id <session-id>`

Resume an existing task by its session ID (UUID). The task's conversation
history is restored so the agent can continue where it left off.

**Default:** None.

```bash
roo --session-id 018f7fc8-7c96-7f7c-98aa-2ec4ff7f6d87 "Continue from where we stopped"
```

#### `-c, --continue`

Resume the most recent task in the current workspace. A shorthand alternative
to looking up the session ID with `roo list sessions` and passing it to
`--session-id`.

**Default:** `false`.

```bash
roo --continue "Pick up where we left off"
roo -c
```

### Execution Mode

#### `-w, --workspace <path>`

Workspace directory path. The agent operates within this directory for file
reads, writes, and command execution.

**Default:** Current working directory.

```bash
roo -w ~/Documents/my-project "Refactor utils"
roo --workspace /home/user/repo "Run the tests"
```

#### `-p, --print`

Non-interactive (headless) mode. The agent runs the prompt, prints the
response, and exits. All actions are auto-approved. Requires a prompt
(positional or `--prompt-file`).

**Default:** `false` (interactive TUI mode).

```bash
roo --print "Summarize this repository"
roo -p --output-format json "List all exported functions"
```

#### `--stdin-prompt-stream`

Read NDJSON control commands from stdin for programmatic orchestration of a
single long-lived process. Requires `--print` and `--output-format
stream-json`.

NDJSON commands: `start` (with `prompt`, optional `taskId`, `images`),
`message` (with `prompt`, optional `images`), `cancel`, `ping`, `shutdown`.
Lifecycle events (`ack`, `done`, `error`) include `requestId` for correlation.

**Default:** `false`.

```bash
printf '{"command":"start","requestId":"1","prompt":"1+1=?"}\n' | \
  roo --print --stdin-prompt-stream --output-format stream-json
```

#### `--signal-only-exit`

Do not exit from normal completion or errors; only terminate on SIGINT or
SIGTERM. Intended for stdin-stream harnesses where the parent process
controls the lifecycle.

**Default:** `false`.

```bash
roo --print --stdin-prompt-stream --signal-only-exit --output-format stream-json
```

#### `--ephemeral`

Run without persisting any state. Uses a temporary storage directory that is
discarded on exit. Useful for one-off CI tasks or sandboxed runs.

**Default:** `false`.

```bash
roo --ephemeral --print "What version of Node is installed?"
```

#### `--oneshot`

Run interactively (with TUI) but exit automatically upon task completion
instead of waiting for further input.

**Default:** `false`.

```bash
roo --oneshot "Create a TODO.md"
```

#### `--exit-on-error`

Exit immediately on API request errors instead of retrying with exponential
backoff. Useful in CI/CD pipelines where failures should propagate quickly.

**Default:** `false` (retries on errors).

```bash
roo --print --exit-on-error "Run the test suite"
```

### Provider and Model

#### `-k, --api-key <key>`

API key for the selected LLM provider. If omitted, the CLI falls back to the
matching environment variable (see Environment Variables), then to the
credential store from `roo auth login`.

**Default:** From environment variable or credential store.

```bash
roo --api-key sk-or-v1-... --provider openrouter "Hello"
```

#### `--provider <provider>`

API provider to use. Supported values: `roo`, `anthropic`, `openai-native`,
`openrouter`, `gemini`, `vercel-ai-gateway`, `unbound`.

**Default:** `openrouter` (falls back to `roo` when authenticated via
`roo auth login`).

```bash
roo --provider anthropic --model claude-sonnet-4-20250514 "Explain this"
roo --provider roo "Hello"
roo --provider gemini --model gemini-2.5-pro "Summarize"
```

#### `-m, --model <model>`

Model identifier to use. The format depends on the provider. For OpenRouter,
use the full `provider/model` format (e.g., `anthropic/claude-opus-4.6`).

**Default:** `anthropic/claude-opus-4.6`.

```bash
roo --model anthropic/claude-sonnet-4-20250514 "Quick fix"
roo -m gemini-2.5-pro --provider gemini "Summarize"
```

### Agent Behavior

#### `--mode <mode>`

Starting mode for the agent. Built-in modes: `code`, `architect`, `ask`,
`debug`. Custom mode slugs are also supported (defined in `.roomodes` or
workspace settings).

**Default:** `code`.

```bash
roo --mode architect "Design a new auth system"
roo --mode ask "Explain the build pipeline"
roo --mode debug "Why is the test failing?"
```

#### `-r, --reasoning-effort <effort>`

Controls how much reasoning/thinking the model performs. Higher values use
more tokens but produce more thorough analysis.

Supported values: `unspecified`, `disabled`, `none`, `minimal`, `low`,
`medium`, `high`, `xhigh`.

**Default:** `medium`.

```bash
roo --reasoning-effort high "Redesign the auth module"
roo -r low "Quick formatting fix"
```

#### `--consecutive-mistake-limit <n>`

Maximum number of consecutive errors or repetitions before the agent pauses
for human intervention. Set to `0` to disable the limit entirely.

**Default:** `10`.

```bash
roo --consecutive-mistake-limit 5 "Refactor the module"
roo --consecutive-mistake-limit 0 "Allow unlimited retries"
```

#### `--terminal-shell <path>`

Absolute path to the shell executable used for inline terminal command
execution. Overrides the auto-detected system shell.

**Default:** Auto-detected shell (e.g., `/bin/zsh`, `/bin/bash`).

```bash
roo --terminal-shell /bin/bash "Run the build"
roo --terminal-shell /usr/bin/fish "List files"
```

### Output and Debugging

#### `--output-format <format>`

Controls output format when used with `--print`:

- `text` -- Human-readable plain text output.
- `json` -- Single JSON object with all events and the final result at task
  completion.
- `stream-json` -- NDJSON (newline-delimited JSON) for real-time streaming of
  events as they occur.

**Default:** `text`.

```bash
roo --print --output-format json "Summarize this repository"
roo --print --output-format stream-json "List all TODO comments"
```

#### `-d, --debug`

Enable detailed debug output including prompts, internal state, file paths,
and extension host messages. Also enables the debug log file at
`~/.roo/cli-debug.log`.

**Default:** `false`.

```bash
roo --debug "Trace through this function"
roo -d --print "What went wrong?"
```

### Permissions

#### `-a, --require-approval`

Require manual approval before any action (tool execution, command, browser,
MCP) is performed. In `--print` mode, approval prompts appear on stderr. In
interactive TUI mode, followup questions wait for manual input with no
auto-timeout.

**Default:** `false` (all actions auto-approved).

```bash
roo --require-approval "Refactor the utils.ts file"
roo -a -w ~/my-project "Run the deploy script"
```

### Extension Internals

#### `-e, --extension <path>`

Path to the extension bundle directory. Overrides auto-detection of the
bundled extension. Useful for development or testing with a custom build.

**Default:** Auto-detected from the CLI installation directory.

```bash
roo --extension /path/to/custom/bundle "Test the extension"
```


## CLI Options (Quick Reference)

### Subcommands

| Subcommand                   | Description                                       |
|------------------------------|---------------------------------------------------|
| `roo [prompt]`               | Start a session (interactive or print)            |
| `roo auth login`             | Authenticate with Roo Code Cloud (opens browser)  |
| `roo auth logout`            | Clear stored authentication token                 |
| `roo auth status`            | Show current authentication status                |
| `roo list commands`          | List available slash commands                     |
| `roo list modes`             | List available agent modes                        |
| `roo list models`            | List available models for the configured provider |
| `roo list sessions`          | List task sessions for the current workspace      |
| `roo upgrade`                | Upgrade the CLI to the latest version             |

### `list` Subcommand Options

The `roo list` family of subcommands all accept these common flags:

| Flag                     | Description                        | Default     |
|--------------------------|------------------------------------|-------------|
| `-w, --workspace <path>` | Workspace directory path           | CWD         |
| `-e, --extension <path>` | Path to extension bundle directory | Auto-detect |
| `-k, --api-key <key>`    | API key                           | From env    |
| `--format <format>`      | Output format: `json` or `text`   | `json`      |
| `-d, --debug`            | Enable debug output               | `false`     |

```bash
roo list models --format text
roo list sessions -w ~/my-project
roo list commands --format json
```

### `auth` Subcommand Options

| Flag            | Description              | Default |
|-----------------|--------------------------|---------|
| `-v, --verbose` | Enable verbose output    | `false` |

```bash
roo auth login --verbose
roo auth status -v
roo auth logout
```


### Switches (Compact Table)

| Switch                                | Description                                                        | Default                                    |
|---------------------------------------|--------------------------------------------------------------------|--------------------------------------------|
| `[prompt]`                            | Initial prompt (positional argument, optional)                     | None                                       |
| `--prompt-file <path>`                | Read prompt from a file                                            | None                                       |
| `--create-with-session-id <uuid>`     | Create a new task with a specific session ID                       | None (auto-generated)                      |
| `--session-id <uuid>`                 | Resume a specific task by session ID                               | None                                       |
| `-c, --continue`                      | Resume the most recent task in the current workspace               | `false`                                    |
| `-w, --workspace <path>`              | Workspace directory to operate in                                  | Current directory                          |
| `-p, --print`                         | Non-interactive mode; print response and exit                      | `false`                                    |
| `--stdin-prompt-stream`               | Read NDJSON commands from stdin (requires `--print`)               | `false`                                    |
| `--signal-only-exit`                  | Only exit on SIGINT/SIGTERM (for stdin stream harnesses)           | `false`                                    |
| `-e, --extension <path>`              | Path to the extension bundle directory                             | Auto-detected                              |
| `-d, --debug`                         | Enable detailed debug output                                       | `false`                                    |
| `-a, --require-approval`              | Require manual approval before actions execute                     | `false`                                    |
| `--exit-on-error`                     | Exit on API request errors instead of retrying                     | `false`                                    |
| `-k, --api-key <key>`                 | API key for the LLM provider                                       | From environment variable                  |
| `--provider <provider>`               | API provider (`roo`, `anthropic`, `openai-native`, `openrouter`, `gemini`, `vercel-ai-gateway`, `unbound`) | `openrouter` (or `roo` if authenticated) |
| `-m, --model <model>`                 | Model to use                                                       | `anthropic/claude-opus-4.6`                |
| `--mode <mode>`                       | Starting mode (`code`, `architect`, `ask`, `debug`, custom slug)   | `code`                                     |
| `-r, --reasoning-effort <effort>`     | Reasoning effort level                                             | `medium`                                   |
| `--consecutive-mistake-limit <n>`     | Consecutive error limit before pause (`0` disables)                | `10`                                       |
| `--terminal-shell <path>`             | Absolute shell path for command execution                          | Auto-detected shell                        |
| `--ephemeral`                         | Run without persisting state (temporary storage)                   | `false`                                    |
| `--oneshot`                           | Exit upon task completion                                          | `false`                                    |
| `--output-format <format>`            | Output format with `--print`: `text`, `json`, `stream-json`        | `text`                                     |

### Environment Variables

| Provider            | Environment Variable          |
|---------------------|-------------------------------|
| roo                 | `ROO_API_KEY`                 |
| anthropic           | `ANTHROPIC_API_KEY`           |
| openai-native       | `OPENAI_API_KEY`              |
| openrouter          | `OPENROUTER_API_KEY`          |
| gemini              | `GOOGLE_API_KEY`              |
| vercel-ai-gateway   | `VERCEL_AI_GATEWAY_API_KEY`   |

| Variable                      | Description                                                        |
|-------------------------------|--------------------------------------------------------------------|
| `ROO_WEB_APP_URL`             | Override the Roo Code Cloud URL (default: `https://app.roocode.com`) |
| `ROO_CODE_PROVIDER_URL`       | Override the provider proxy URL (default: `https://api.roocode.com/proxy`) |
| `ROO_INSTALL_DIR`             | Custom installation directory for the CLI binary                   |
| `ROO_BIN_DIR`                 | Custom bin directory for the `roo` symlink                         |
| `ROO_VERSION`                 | Pin a specific CLI version during install                          |
| `ROO_CODE_DISABLE_TELEMETRY`  | Set to `1` to disable cloud telemetry                              |
| `ROO_LOCAL_TARBALL`           | Install from a local tarball instead of downloading (for offline)  |
| `ROO_AUTH_BASE_URL`           | Override auth base URL (development)                               |
| `ROO_SDK_BASE_URL`            | Override SDK base URL (development)                                |


## Sources

- [Roo Code Homepage](https://roocode.com/)
- [Roo Code Documentation](https://docs.roocode.com/)
- [Sunsetting Roo Code](https://docs.roocode.com/sunset)
- [Roo Code CLI README (apps/cli)](https://github.com/RooCodeInc/Roo-Code/tree/main/apps/cli)
- [Roo Code CLI Source (index.ts)](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/index.ts)
- [Roo Code CLI Changelog](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/CHANGELOG.md)
- [Roo Code GitHub Repository](https://github.com/RooCodeInc/Roo-Code)
- [Custom Instructions](https://docs.roocode.com/features/custom-instructions)
- [Footgun Prompting: Override System Prompts](https://docs.roocode.com/advanced-usage/footgun-prompting)
- [Auto-Approving Actions](https://docs.roocode.com/features/auto-approving-actions)
- [API Configuration Profiles](https://docs.roocode.com/features/api-configuration-profiles)
- [Customizing Modes](https://docs.roocode.com/features/custom-modes)
- [Boomerang Tasks](https://docs.roocode.com/features/boomerang-tasks)
- [Roo Code Cloud Pricing](https://roocode.com/pricing)
- [Settings Management](https://docs.roocode.com/features/settings-management)
- [CLI/Headless Execution Issue #3835](https://github.com/RooCodeInc/Roo-Code/issues/3835)
- [CLI Releases on GitHub](https://github.com/RooCodeInc/Roo-Code/releases)
