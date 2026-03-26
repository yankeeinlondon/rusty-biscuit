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

### Stdin Stream Mode (`--stdin-prompt-stream`)

Pipe multiple prompts, one per line, through a single process.

```bash
printf '1+1=?\n10!=?\n' | roo --print --stdin-prompt-stream --output-format stream-json
```

- Requires `--print`.
- Each line is processed as a separate prompt; subsequent prompts reuse the
  existing task rather than creating new ones.

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


## CLI Options

### Subcommands

| Subcommand        | Description                              |
|-------------------|------------------------------------------|
| `roo [prompt]`    | Start a session (interactive or print)   |
| `roo auth login`  | Authenticate with Roo Code Cloud         |
| `roo auth logout` | Clear stored authentication token        |
| `roo auth status` | Show current authentication status       |

### Switches

| Switch                              | Description                                                        | Default                                    |
|-------------------------------------|--------------------------------------------------------------------|--------------------------------------------|
| `[prompt]`                          | Initial prompt (positional argument, optional)                     | None                                       |
| `--prompt-file <path>`              | Read prompt from a file                                            | None                                       |
| `-w, --workspace <path>`            | Workspace directory to operate in                                  | Current directory                          |
| `-p, --print`                       | Non-interactive mode; print response and exit                      | `false`                                    |
| `--stdin-prompt-stream`             | Read prompts from stdin, one per line (requires `--print`)         | `false`                                    |
| `-e, --extension <path>`            | Path to the extension bundle directory                             | Auto-detected                              |
| `-d, --debug`                       | Enable detailed debug output                                       | `false`                                    |
| `-a, --require-approval`            | Require manual approval before actions execute                     | `false`                                    |
| `-k, --api-key <key>`               | API key for the LLM provider                                       | From environment variable                  |
| `--provider <provider>`             | API provider (`roo`, `anthropic`, `openai-native`, `openrouter`, `gemini`, `vercel-ai-gateway`) | `openrouter` (or `roo` if authenticated) |
| `-m, --model <model>`               | Model to use                                                       | `anthropic/claude-opus-4.6`                |
| `--mode <mode>`                     | Starting mode (`code`, `architect`, `ask`, `debug`, custom slug)   | `code`                                     |
| `-r, --reasoning-effort <effort>`   | Reasoning effort level (`unspecified`, `disabled`, `none`, `minimal`, `low`, `medium`, `high`, `xhigh`) | `medium` |
| `--ephemeral`                       | Run without persisting state (temporary storage)                   | `false`                                    |
| `--oneshot`                         | Exit upon task completion                                          | `false`                                    |
| `--output-format <format>`          | Output format with `--print`: `text`, `json`, `stream-json`       | `text`                                     |

### Environment Variables

| Provider            | Environment Variable          |
|---------------------|-------------------------------|
| roo                 | `ROO_API_KEY`                 |
| anthropic           | `ANTHROPIC_API_KEY`           |
| openai-native       | `OPENAI_API_KEY`              |
| openrouter          | `OPENROUTER_API_KEY`          |
| gemini              | `GOOGLE_API_KEY`              |
| vercel-ai-gateway   | `VERCEL_AI_GATEWAY_API_KEY`   |

| Variable            | Description                                                  |
|---------------------|--------------------------------------------------------------|
| `ROO_WEB_APP_URL`   | Override the Roo Code Cloud URL (default: `https://app.roocode.com`) |
| `ROO_INSTALL_DIR`   | Custom installation directory for the CLI binary             |
| `ROO_BIN_DIR`       | Custom bin directory for the `roo` symlink                   |
| `ROO_VERSION`       | Pin a specific CLI version during install                    |


## Sources

- [Roo Code Homepage](https://roocode.com/)
- [Roo Code Documentation](https://docs.roocode.com/)
- [Roo Code CLI README (apps/cli)](https://github.com/RooCodeInc/Roo-Code/tree/main/apps/cli)
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
