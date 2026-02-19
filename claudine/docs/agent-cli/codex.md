---
homepage: https://github.com/openai/codex
docs: https://developers.openai.com/codex/cli/
cli_docs: https://developers.openai.com/codex/cli/reference/
---

# Codex CLI

OpenAI Codex CLI is an open-source, Rust-based coding agent that runs locally in the terminal. It can read, change, and run code on the host machine. Codex is included with ChatGPT Plus, Pro, Business, Edu, and Enterprise plans, or can be used with an API key at standard per-token rates. Binary name: `codex`. Configuration lives in `~/.codex/config.toml` (TOML format).

Repository: https://github.com/openai/codex

---

## Model Specification

**CLI flag:** `-m` / `--model <MODEL>`

```bash
codex -m gpt-5.3-codex "refactor the auth module"
codex exec -m o3 "summarize the repo"
```

**Config file (`~/.codex/config.toml`):**

```toml
model = "gpt-5.3-codex"
```

**Default model:** If no model is specified via CLI flag or config file, Codex automatically defaults to the current recommended model (currently `gpt-5.3-codex`). The config key `model` sets the persistent default. When OpenAI deprecates a model, Codex performs automatic migration and records it under `[notice.model_migrations]` in `config.toml`.

**Interactive switching:** Use the `/model` slash command during an active TUI session to change the model mid-session.

**Local/OSS models:** The `--oss` flag selects a local open-source model provider (LM Studio or Ollama). Combine with `--local-provider <lmstudio|ollama>` to specify which. Equivalent config:

```toml
model_provider = "oss"
```

**Profiles:** Named profiles under `[profiles.<name>]` can set different models per workflow, activated with `-p <name>` / `--profile <name>`.

---

## Non-interactive Engagement

Non-interactive mode is fully supported through the `codex exec` subcommand (alias: `codex e`). There are several ways to run non-interactively:

### 1. `codex exec` with inline prompt

The primary non-interactive method. Streams progress to stderr; writes only the final agent message to stdout.

```bash
codex exec "summarize the repository structure"
codex exec "generate release notes" | tee release-notes.md
```

### 2. Piping prompt from stdin

When the prompt argument is omitted or set to `-`, instructions are read from stdin.

```bash
echo "explain the auth flow" | codex exec -
cat instructions.txt | codex exec
```

### 3. JSONL event stream (`--json`)

Produces machine-readable newline-delimited JSON events (thread.started, turn.started, turn.completed, item.*) suitable for programmatic consumption.

```bash
codex exec --json "summarize the repo" | jq
```

### 4. Output to file (`-o` / `--output-last-message`)

Writes the agent's final message to a specified file, useful for downstream scripting.

```bash
codex exec -o result.md "triage open issues"
```

### 5. Structured output (`--output-schema`)

Constrains the final response to match a JSON Schema, producing validated structured data.

```bash
codex exec --output-schema ./schema.json -o ./metadata.json "extract project metadata"
```

### 6. Ephemeral mode (`--ephemeral`)

Runs without persisting session files to disk. Useful in CI/CD where session history is unwanted.

```bash
codex exec --ephemeral "run tests and report"
```

### 7. Code review (`codex review` / `codex exec review`)

Dedicated non-interactive code review. Both top-level `codex review` and `codex exec review` are available.

```bash
codex review --uncommitted
codex review --base main
codex exec review --commit abc123
```

### 8. Session resume in exec mode

Continue a previous session non-interactively:

```bash
codex exec resume --last "fix the race conditions you found"
codex exec resume <SESSION_ID> "apply the suggested changes"
```

**Limitations:**
- The `--search` flag (live web search) is only available in interactive TUI mode, not in `exec`.
- The `--ask-for-approval` / `-a` flag is not available on `exec` (approval policy is effectively `never` since there is no human present).

---

## Subscription versus Per Call API

Codex CLI supports two authentication and billing models:

**Subscription (ChatGPT sign-in):** Authenticate via `codex login` using ChatGPT OAuth or device auth. Usage is included in your ChatGPT Plus ($20/month), Pro ($200/month), Business ($30/user/month), or Enterprise plan. Subscription plans include usage limits within shared five-hour windows. Local tasks average ~5 credits per message.

**Per-call API key:** Authenticate with an API key via `codex login --with-api-key` or set the `CODEX_API_KEY` environment variable (primarily for `codex exec` in CI/CD). API usage is billed at standard per-token rates (e.g., GPT-5.3-Codex: ~$1.25/1M input tokens, ~$10/1M output tokens). API key access does not include cloud features (GitHub code review integrations, Slack).

**Launching non-interactive sessions:**

```bash
# Subscription (requires prior `codex login`)
codex exec "your task here"

# API key for CI/CD
CODEX_API_KEY=sk-... codex exec --json "triage open bug reports"
```

Both modes use the same CLI interface; the billing model is determined by the authentication method.

---

## System Prompt

Codex uses a layered instruction system built from multiple sources, concatenated at session start:

### AGENTS.md (primary project instructions)

Codex discovers and concatenates `AGENTS.md` files walking from the git root to the current directory. Files closer to the working directory override earlier ones.

- **Global scope:** `~/.codex/AGENTS.override.md`, then `~/.codex/AGENTS.md`
- **Project scope:** Walk from git root to cwd, checking override files first, then regular files
- Maximum size controlled by `project_doc_max_bytes` (default: 32 KiB)
- Fallback filenames configurable via `project_doc_fallback_filenames`

### model_instructions_file (full replacement)

Replaces the built-in instructions entirely with a custom file:

```toml
model_instructions_file = "custom-instructions.md"
```

Relative paths are resolved relative to the `.codex/` directory containing the config.

### developer_instructions (inline supplement)

Additional developer instructions injected into the session context:

```toml
developer_instructions = "Always use TypeScript strict mode. Prefer functional patterns."
```

### Personality

Controls communication style for models that support it:

```toml
personality = "pragmatic"  # "none" | "friendly" | "pragmatic"
```

### Precedence

The instruction chain is rebuilt on every run. Empty files are skipped. The merge order is: global override -> global base -> project root -> subdirectories (closest to cwd wins).

---

## Permissions

Codex uses two independent permission axes: **sandbox policy** (filesystem/network access) and **approval policy** (human confirmation).

### Sandbox Policy

Controls what the agent's shell commands can access. Set via `-s` / `--sandbox` or config key `sandbox_mode`.

| Value | Behavior |
|---|---|
| `read-only` | Default. No filesystem writes, no network. |
| `workspace-write` | Write access to workspace + `$TMPDIR`. Network configurable. |
| `danger-full-access` | Unrestricted filesystem and network. |

Fine-tune `workspace-write` in config:

```toml
[sandbox_workspace_write]
network_access = true
writable_roots = ["/tmp/build-output"]
exclude_slash_tmp = false
```

### Approval Policy

Controls when Codex pauses for human confirmation. Set via `-a` / `--ask-for-approval` or config key `approval_policy`.

| Value | Behavior |
|---|---|
| `untrusted` | Only trusted commands (ls, cat, sed, etc.) run automatically; others prompt. |
| `on-failure` | All commands run automatically; prompts only on execution failure. |
| `on-request` | The model decides when to ask. |
| `never` | Never prompts. Failures go directly back to the model. |

### Convenience aliases

| Flag | Equivalent |
|---|---|
| `--full-auto` | `-a on-request --sandbox workspace-write` |
| `--dangerously-bypass-approvals-and-sandbox` (alias: `--yolo`) | Skip all confirmations and sandboxing. Only for externally sandboxed environments. |

### Execution Rules

Codex supports Starlark-based rule files that control which commands can run outside the sandbox:

- **User rules:** `~/.codex/rules/default.rules`
- **Project rules:** `.codex/rules/` directory
- Decisions: `allow`, `prompt`, `forbidden` (most restrictive wins)
- Test rules: `codex execpolicy check --pretty --rules <file> -- <command>`

### Trust levels

Per-project trust in config:

```toml
[projects."/path/to/repo"]
trust_level = "trusted"
```

---

## Thinking Level

Codex supports configurable reasoning effort for models that use the Responses API.

**Config key:** `model_reasoning_effort`

```toml
model_reasoning_effort = "high"
```

**Supported levels:** `minimal`, `low`, `medium`, `high`, `xhigh`

**CLI override:**

```bash
codex -c model_reasoning_effort='"xhigh"' "solve this complex bug"
codex exec -c model_reasoning_effort='"low"' "quick formatting fix"
```

**Related config keys:**

| Key | Values | Description |
|---|---|---|
| `model_reasoning_effort` | `minimal` / `low` / `medium` / `high` / `xhigh` | Reasoning depth for supported models |
| `model_reasoning_summary` | `auto` / `concise` / `detailed` / `none` | Reasoning summary detail level |
| `model_supports_reasoning_summaries` | `true` / `false` | Force reasoning metadata on/off |
| `model_verbosity` | `low` / `medium` / `high` | Response verbosity (Responses API) |
| `show_raw_agent_reasoning` | `true` / `false` | Surface raw reasoning content |
| `hide_agent_reasoning` | `true` / `false` | Suppress reasoning events in TUI and exec output |

These can also be set per-profile:

```toml
[profiles.fast]
model_reasoning_effort = "low"
model_verbosity = "low"

[profiles.deep]
model_reasoning_effort = "xhigh"
model_reasoning_summary = "detailed"
```

---

## Logging

### TUI log

The interactive TUI writes a log file at:

```
~/.codex/log/codex-tui.log
```

### Session transcripts

Session history is stored in a date-partitioned directory structure:

```
~/.codex/sessions/<YYYY>/<MM>/<DD>/<session-uuid>/
```

Controlled by config:

```toml
[history]
persistence = "save-all"   # "save-all" | "none"
max_bytes = 10485760       # optional cap on history.jsonl
```

### History file

Conversation summaries are appended to:

```
~/.codex/history.jsonl
```

### Shell snapshots

Shell environment snapshots are stored in:

```
~/.codex/shell_snapshots/
```

### OpenTelemetry

For production observability, Codex supports OpenTelemetry export:

```toml
[otel]
exporter = "otlp-http"      # "none" | "otlp-http" | "otlp-grpc"
environment = "production"
log_user_prompt = false
```

---

## CLI Options

### Subcommands

| Subcommand | Alias | Maturity | Description |
|---|---|---|---|
| *(default)* | | Stable | Launch the interactive terminal UI (TUI) |
| `exec` | `e` | Stable | Run Codex non-interactively; streams results to stdout or JSONL |
| `exec resume` | | Stable | Resume a previous session non-interactively |
| `exec review` | | Stable | Run a code review non-interactively |
| `review` | | Stable | Run a code review non-interactively (top-level shorthand) |
| `apply` | `a` | Stable | Apply the latest diff from a Codex Cloud task via `git apply` |
| `resume` | | Stable | Resume a previous interactive session (picker or `--last`) |
| `fork` | | Stable | Fork a previous interactive session into a new thread |
| `login` | | Stable | Authenticate via ChatGPT OAuth, device auth, or API key |
| `login status` | | Stable | Show current login status |
| `logout` | | Stable | Remove stored authentication credentials |
| `completion` | | Stable | Generate shell completion scripts (bash, zsh, fish, elvish, powershell) |
| `features` | | Stable | Inspect and manage feature flags |
| `features list` | | Stable | List known features with stage and effective state |
| `features enable` | | Stable | Enable a feature in config.toml |
| `features disable` | | Stable | Disable a feature in config.toml |
| `cloud` | | Experimental | Browse Codex Cloud tasks and apply changes locally |
| `cloud exec` | | Experimental | Submit a new Codex Cloud task without the TUI |
| `cloud status` | | Experimental | Show the status of a Codex Cloud task |
| `cloud list` | | Experimental | List Codex Cloud tasks |
| `cloud apply` | | Experimental | Apply the diff for a Codex Cloud task locally |
| `cloud diff` | | Experimental | Show the unified diff for a Codex Cloud task |
| `mcp` | | Experimental | Manage MCP (Model Context Protocol) servers |
| `mcp list` | | Experimental | List configured MCP servers |
| `mcp get` | | Experimental | Get details for an MCP server |
| `mcp add` | | Experimental | Add an MCP server |
| `mcp remove` | | Experimental | Remove an MCP server |
| `mcp login` | | Experimental | Authenticate with an MCP server |
| `mcp logout` | | Experimental | Remove MCP server credentials |
| `mcp-server` | | Experimental | Run Codex as an MCP server (stdio transport) |
| `app` | | Stable | Launch the Codex desktop app (macOS; downloads installer if missing) |
| `app-server` | | Experimental | Run the app server or related tooling |
| `app-server generate-ts` | | Experimental | Generate TypeScript bindings for the app server protocol |
| `app-server generate-json-schema` | | Experimental | Generate JSON Schema for the app server protocol |
| `sandbox` | | Experimental | Run commands within a Codex-provided sandbox |
| `sandbox macos` | `seatbelt` | Experimental | Run a command under macOS Seatbelt |
| `sandbox linux` | `landlock` | Experimental | Run a command under Linux Landlock+seccomp |
| `sandbox windows` | | Experimental | Run a command under Windows restricted token |
| `debug` | | Stable | Debugging tools |
| `debug app-server` | | Stable | Debug the app server |

### Global switches (available on the default TUI, `resume`, and `fork`)

| Switch | Short | Values / Type | Description |
|---|---|---|---|
| `--model` | `-m` | `<MODEL>` | Override configured model |
| `--config` | `-c` | `<key=value>` | Override a config.toml value (TOML syntax; dotted paths for nesting) |
| `--profile` | `-p` | `<NAME>` | Use a named configuration profile |
| `--image` | `-i` | `<FILE>...` | Attach image(s) to the initial prompt |
| `--sandbox` | `-s` | `read-only` / `workspace-write` / `danger-full-access` | Sandbox policy for shell commands |
| `--ask-for-approval` | `-a` | `untrusted` / `on-failure` / `on-request` / `never` | When to require human approval |
| `--full-auto` | | flag | Alias for `-a on-request --sandbox workspace-write` |
| `--dangerously-bypass-approvals-and-sandbox` | | flag | Skip all confirmations and sandboxing (`--yolo` alias) |
| `--cd` | `-C` | `<DIR>` | Set working directory for the agent |
| `--add-dir` | | `<DIR>` | Grant additional directories write access |
| `--search` | | flag | Enable live web search (native Responses `web_search` tool) |
| `--oss` | | flag | Use local OSS model provider (LM Studio or Ollama) |
| `--local-provider` | | `lmstudio` / `ollama` | Specify which local provider |
| `--enable` | | `<FEATURE>` | Enable a feature flag (repeatable) |
| `--disable` | | `<FEATURE>` | Disable a feature flag (repeatable) |
| `--no-alt-screen` | | flag | Disable alternate screen (inline TUI mode; useful in Zellij) |
| `--help` | `-h` | flag | Print help |
| `--version` | `-V` | flag | Print version |

### `exec`-specific switches

| Switch | Short | Values / Type | Description |
|---|---|---|---|
| `--json` | | flag | Print JSONL events to stdout |
| `--output-last-message` | `-o` | `<FILE>` | Write the agent's final message to a file |
| `--output-schema` | | `<FILE>` | JSON Schema file to validate the final response shape |
| `--color` | | `always` / `never` / `auto` | ANSI color in stdout (default: `auto`) |
| `--ephemeral` | | flag | Do not persist session files to disk |
| `--skip-git-repo-check` | | flag | Allow running outside a Git repository |

### `review`-specific switches

| Switch | Values / Type | Description |
|---|---|---|
| `--uncommitted` | flag | Review staged, unstaged, and untracked changes |
| `--base` | `<BRANCH>` | Review changes against a base branch |
| `--commit` | `<SHA>` | Review the changes introduced by a specific commit |
| `--title` | `<TITLE>` | Optional commit title for the review summary |

### `resume` / `fork`-specific switches

| Switch | Values / Type | Description |
|---|---|---|
| `--last` | flag | Select the most recent session without showing the picker |
| `--all` | flag | Show all sessions (disables cwd filtering) |

### `login`-specific switches

| Switch | Description |
|---|---|
| `--with-api-key` | Read API key from stdin (e.g., `printenv OPENAI_API_KEY \| codex login --with-api-key`) |
| `--device-auth` | Use device code authentication for headless environments |

### `cloud exec`-specific switches

| Switch | Values / Type | Description |
|---|---|---|
| `--env` | `<ENV_ID>` | Target environment identifier (required) |
| `--attempts` | `<N>` | Number of assistant attempts, best-of-N (default: 1) |
| `--branch` | `<BRANCH>` | Git branch to run in Codex Cloud (default: current branch) |

---

## Sources

- [Codex CLI GitHub Repository](https://github.com/openai/codex)
- [Codex CLI Features](https://developers.openai.com/codex/cli/features/)
- [Codex CLI Command Line Options Reference](https://developers.openai.com/codex/cli/reference/)
- [Codex Config Basics](https://developers.openai.com/codex/config-basic/)
- [Codex Advanced Configuration](https://developers.openai.com/codex/config-advanced/)
- [Codex Configuration Reference](https://developers.openai.com/codex/config-reference/)
- [Codex Sample Configuration](https://developers.openai.com/codex/config-sample/)
- [Codex Non-interactive Mode](https://developers.openai.com/codex/noninteractive/)
- [Codex Models](https://developers.openai.com/codex/models/)
- [Codex Pricing](https://developers.openai.com/codex/pricing/)
- [Codex Rules](https://developers.openai.com/codex/rules/)
- [Codex AGENTS.md](https://developers.openai.com/codex/guides/agents-md/)
- [Codex Authentication](https://developers.openai.com/codex/auth/)
