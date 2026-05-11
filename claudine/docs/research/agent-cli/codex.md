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
codex -m gpt-5.5 "refactor the auth module"
codex exec -m gpt-5.4 "summarize the repo"
```

**Config file (`~/.codex/config.toml`):**

```toml
model = "gpt-5.5"
```

**Default model:** If no model is specified via CLI flag or config file, Codex automatically defaults to the current recommended model (currently `gpt-5.5`). The config key `model` sets the persistent default. When OpenAI deprecates a model, Codex performs automatic migration and records it under `[notice.model_migrations]` in `config.toml`.

**Recommended models:**

| Model | Use case |
|---|---|
| `gpt-5.5` | Primary model for complex coding, computer use, knowledge work, and research (newest frontier) |
| `gpt-5.4` | Fallback when `gpt-5.5` is not yet available |
| `gpt-5.3-codex-spark` | Extra fast tasks (ChatGPT Pro subscribers, research preview) |

**Interactive switching:** Use the `/model` slash command during an active TUI session to change the model mid-session.

**Local/OSS models:** The `--oss` flag selects a local open-source model provider (requires Ollama running). Equivalent config:

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

### 7. Session resume in exec mode

Continue a previous session non-interactively:

```bash
codex exec resume --last "fix the race conditions you found"
codex exec resume <SESSION_ID> "apply the suggested changes"
codex exec resume --all --last "continue from any directory"
```

**Limitations:**
- The `--search` flag (live web search) is only available in interactive TUI mode, not in `exec`.
- The `--ask-for-approval` / `-a` flag is not available on `exec` (approval policy is effectively `never` since there is no human present).

---

## Subscription versus Per Call API

Codex CLI supports two authentication and billing models:

**Subscription (ChatGPT sign-in):** Authenticate via `codex login` using ChatGPT OAuth or device auth. Usage is included in your ChatGPT Plus ($20/month), Pro ($200/month), Business ($30/user/month), or Enterprise plan. Subscription plans include usage limits within shared five-hour windows. Local tasks average ~5 credits per message.

**Per-call API key:** Authenticate with an API key via `codex login --with-api-key` or set the `CODEX_API_KEY` environment variable (primarily for `codex exec` in CI/CD). API usage is billed at standard per-token rates. API key access does not include cloud features (GitHub code review integrations, Slack).

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
| `workspace-write` | Write access to workspace + `$TMPDIR` + `~/.codex/memories`. Network configurable. |
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
| `on-request` | The model decides when to ask. Recommended for interactive runs. |
| `never` | Never prompts. Failures go directly back to the model. Recommended for non-interactive runs. |

> **Note:** `on-failure` is deprecated; prefer `on-request` for interactive runs or `never` for non-interactive runs.

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

## CLI Switch Summary

### Subcommands

| Subcommand | Alias | Maturity | Description |
|---|---|---|---|
| *(default)* | | Stable | Launch the interactive terminal UI (TUI) |
| `exec` | `e` | Stable | Run Codex non-interactively; streams results to stdout or JSONL |
| `exec resume` | | Stable | Resume a previous exec session non-interactively |
| `apply` | `a` | Stable | Apply the latest diff from a Codex Cloud task via `git apply` |
| `resume` | | Stable | Resume a previous interactive session (picker or `--last`) |
| `fork` | | Stable | Fork a previous interactive session into a new thread |
| `login` | | Stable | Authenticate via ChatGPT OAuth, device auth, or API key |
| `login status` | | Stable | Show current login status |
| `logout` | | Stable | Remove stored authentication credentials |
| `completion` | | Stable | Generate shell completion scripts (bash, zsh, fish, power-shell, elvish) |
| `features` | | Stable | Inspect and manage feature flags |
| `features list` | | Stable | List known features with stage and effective state |
| `features enable` | | Stable | Enable a feature in config.toml |
| `features disable` | | Stable | Disable a feature in config.toml |
| `app` | | Stable | Launch the Codex desktop app (macOS/Windows) |
| `cloud` | | Experimental | Browse Codex Cloud tasks interactively |
| `cloud exec` | | Experimental | Submit a new Codex Cloud task without the TUI |
| `cloud list` | | Experimental | List Codex Cloud tasks with filtering and pagination |
| `app-server` | | Experimental | Launch the Codex app server for local development or debugging |
| `mcp` | | Experimental | Manage MCP (Model Context Protocol) servers |
| `mcp list` | | Experimental | List configured MCP servers |
| `mcp get` | | Experimental | Get details for an MCP server |
| `mcp add` | | Experimental | Add an MCP server |
| `mcp remove` | | Experimental | Remove an MCP server |
| `mcp login` | | Experimental | Authenticate with an MCP server (OAuth) |
| `mcp logout` | | Experimental | Remove MCP server credentials |
| `mcp-server` | | Experimental | Run Codex as an MCP server (stdio transport) |
| `execpolicy` | | Experimental | Evaluate execpolicy rule files |
| `execpolicy check` | | Experimental | Check whether a command would be allowed, prompted, or blocked |
| `sandbox` | | Experimental | Run commands within a Codex-provided sandbox |
| `sandbox macos` | | Experimental | Run a command under macOS Seatbelt |
| `sandbox linux` | | Experimental | Run a command under Linux Landlock+seccomp |
| `sandbox windows` | | Experimental | Run a command under Windows restricted token |
| `plugin marketplace` | | Experimental | Manage plugin marketplaces |
| `plugin marketplace add` | | Experimental | Add a plugin marketplace (Git or local source) |
| `plugin marketplace remove` | | Experimental | Remove a plugin marketplace |
| `plugin marketplace upgrade` | | Experimental | Refresh one or all Git marketplaces |
| `debug app-server send-message-v2` | | Experimental | Send one message through app-server's V2 test client |

### Global switches

Available on the default TUI command, `resume`, and `fork`. Place global flags after the subcommand.

#### `--model` / `-m` `<MODEL>`

Override the model set in configuration.

- **Default:** The value of `model` in `config.toml` (currently defaults to `gpt-5.5`)
- **Example:**

```bash
codex -m gpt-5.5 "refactor the auth module"
codex exec -m gpt-5.4 "summarize the repo"
```

#### `--config` / `-c` `<key=value>`

Override a configuration value for this invocation. Values parse as JSON if possible; otherwise the literal string is used. Repeatable.

- **Default:** Values from `~/.codex/config.toml`
- **Example:**

```bash
codex -c model_reasoning_effort='"xhigh"' "complex task"
codex exec -c sandbox_mode='"workspace-write"' "fix tests"
```

#### `--profile` / `-p` `<NAME>`

Configuration profile name to load from `~/.codex/config.toml`.

- **Default:** No profile (uses root config)
- **Example:**

```bash
codex -p fast "quick formatting fix"
codex -p deep "architect a new module"
```

#### `--image` / `-i` `<path[,path...]>`

Attach one or more image files to the initial prompt. Separate multiple paths with commas or repeat the flag.

- **Default:** No images
- **Example:**

```bash
codex -i screenshot.png "Explain this error"
codex -i img1.png,img2.jpg "Summarize these diagrams"
codex exec -i mockup.png "implement this design"
```

#### `--sandbox` / `-s` `<read-only | workspace-write | danger-full-access>`

Select the sandbox policy for model-generated shell commands.

- **Default:** `read-only` (unless overridden by `--full-auto` or config)
- **Example:**

```bash
codex --sandbox read-only "explore the codebase"
codex --sandbox workspace-write "fix the failing tests"
codex --sandbox danger-full-access "run the deployment"
```

#### `--ask-for-approval` / `-a` `<untrusted | on-request | never>`

Control when Codex pauses for human approval before running a command.

- **Default:** Varies by mode (interactive: typically `untrusted`; non-interactive: effectively `never`)
- **Note:** `on-failure` is deprecated; prefer `on-request` for interactive runs or `never` for non-interactive runs.
- **Example:**

```bash
codex -a untrusted "explore the repo"
codex -a on-request "refactor the module"
codex -a never "batch fix lint errors"
```

#### `--full-auto`

Shortcut for low-friction local work: sets `--ask-for-approval on-request` and `--sandbox workspace-write`.

- **Default:** Off
- **Example:**

```bash
codex --full-auto "fix all lint warnings"
codex exec --full-auto "run the test suite and fix failures"
```

#### `--dangerously-bypass-approvals-and-sandbox` / `--yolo`

Run every command without approvals or sandboxing. Only use inside an externally hardened environment.

- **Default:** Off
- **Example:**

```bash
codex --yolo "set up the development environment"
codex exec --yolo "install deps and run migrations"
```

#### `--cd` / `-C` `<path>`

Set the working directory for the agent before it starts processing your request.

- **Default:** Current working directory
- **Example:**

```bash
codex --cd ~/projects/my-app "explain the architecture"
codex exec --cd ~/projects/api "generate openapi spec"
```

#### `--add-dir` `<path>`

Grant additional directories write access alongside the main workspace. Repeat for multiple paths.

- **Default:** Only the workspace root is writable (in `workspace-write` mode)
- **Example:**

```bash
codex --cd apps/frontend --add-dir ../backend --add-dir ../shared "update the API client"
```

#### `--search`

Enable live web search (sets `web_search = "live"` instead of the default `"cached"`).

- **Default:** Web search uses cached mode (pre-indexed results from an OpenAI-maintained index). When using `--yolo` or another full-access sandbox setting, web search defaults to live results.
- **Config alternative:** `web_search = "live"` or `web_search = "disabled"` in config.toml
- **Example:**

```bash
codex --search "find the latest React best practices"
```

#### `--oss`

Use the local open source model provider. Equivalent to `-c model_provider="oss"`. Validates that Ollama is running.

- **Default:** Off (uses OpenAI API)
- **Example:**

```bash
codex --oss "explain this function"
```

#### `--enable` `<feature>`

Force-enable a feature flag. Translates to `-c features.<name>=true`. Repeatable.

- **Default:** Feature flags from config
- **Example:**

```bash
codex --enable unified_exec "run the migration"
```

#### `--disable` `<feature>`

Force-disable a feature flag. Translates to `-c features.<name>=false`. Repeatable.

- **Default:** Feature flags from config
- **Example:**

```bash
codex --disable shell_snapshot "debug a timing issue"
```

#### `--no-alt-screen`

Disable alternate screen mode for the TUI. Overrides `tui.alternate_screen` for this run. Useful inside multiplexers like Zellij where alternate screen can cause rendering issues.

- **Default:** Alternate screen enabled
- **Example:**

```bash
codex --no-alt-screen
```

#### `--remote` `<ws://host:port | wss://host:port>`

Connect the interactive TUI to a remote app-server WebSocket endpoint. Supported for `codex`, `codex resume`, and `codex fork`; other subcommands reject remote mode.

- **Default:** No remote connection (local mode)
- **Example:**

```bash
codex --remote ws://127.0.0.1:4500
codex --remote wss://codex-devbox.example.com:4500 --remote-auth-token-env CODEX_REMOTE_AUTH_TOKEN
```

#### `--remote-auth-token-env` `<ENV_VAR>`

Read a bearer token from this environment variable and send it when connecting with `--remote`. Tokens are only sent over `wss://` URLs or `ws://` URLs whose host is `localhost`, `127.0.0.1`, or `::1`.

- **Default:** No auth token
- **Requires:** `--remote`
- **Example:**

```bash
export CODEX_REMOTE_AUTH_TOKEN="$(ssh devbox 'cat ~/.codex/codex-app-server-token')"
codex --remote wss://codex-devbox.example.com:4500 \
  --remote-auth-token-env CODEX_REMOTE_AUTH_TOKEN
```

#### `--help` / `-h`

Print help text and exit.

#### `--version` / `-V`

Print version and exit.

#### Positional: `PROMPT`

Optional text instruction to start the session. Omit to launch the TUI without a pre-filled message.

```bash
codex "Explain this codebase to me"
codex
```

### `codex exec` switches

Run Codex non-interactively (alias: `codex e`).

#### `--json` / `--experimental-json`

Print newline-delimited JSON events instead of formatted text.

- **Default:** Formatted text output
- **Example:**

```bash
codex exec --json "summarize the repo" | jq
```

#### `--output-last-message` / `-o` `<path>`

Write the agent's final message to a file. Useful for downstream scripting.

- **Default:** Final message written to stdout only
- **Example:**

```bash
codex exec -o result.md "triage open issues"
codex exec --json -o summary.txt "generate release notes"
```

#### `--output-schema` `<path>`

JSON Schema file describing the expected final response shape. Codex validates tool output against it.

- **Default:** No schema validation
- **Example:**

```bash
codex exec --output-schema ./schema.json -o ./metadata.json "extract project metadata"
```

#### `--color` `<always | never | auto>`

Control ANSI color in stdout.

- **Default:** `auto` (colors when stdout is a TTY)
- **Example:**

```bash
codex exec --color never "fix lint" > output.txt
codex exec --color always "explain module" | less -R
```

#### `--ephemeral`

Run without persisting session rollout files to disk. Useful in CI/CD.

- **Default:** Session files persisted to `~/.codex/sessions/`
- **Example:**

```bash
codex exec --ephemeral "run tests and report"
```

#### `--skip-git-repo-check`

Allow running outside a Git repository (useful for one-off directories).

- **Default:** Codex requires a Git repository
- **Example:**

```bash
codex exec --skip-git-repo-check "organize these files"
```

#### `--cd` / `-C` `<path>`

Set the workspace root before executing the task. (Also available as a global flag.)

- **Default:** Current working directory
- **Example:**

```bash
codex exec --cd ~/projects/api "generate docs"
```

#### `--dangerously-bypass-approvals-and-sandbox` / `--yolo`

Bypass approval prompts and sandboxing for non-interactive execution. Dangerous -- only use inside an isolated runner.

- **Default:** Off
- **Example:**

```bash
codex exec --yolo "install deps and run all tests"
```

#### `--full-auto`

Apply the low-friction automation preset (`workspace-write` sandbox and `on-request` approvals) in non-interactive mode.

- **Default:** Off
- **Example:**

```bash
codex exec --full-auto "fix all lint warnings"
```

#### `--image` / `-i` `<path[,path...]>`

Attach images to the first message. Repeatable; supports comma-separated lists. (Also available as a global flag.)

- **Default:** No images
- **Example:**

```bash
codex exec -i mockup.png "implement this design"
```

#### `--model` / `-m` `<string>`

Override the configured model for this run. (Also available as a global flag.)

- **Default:** Value from `config.toml`
- **Example:**

```bash
codex exec -m gpt-5.5 "architect a solution"
```

#### `--oss`

Use the local open source provider (requires a running Ollama instance). (Also available as a global flag.)

- **Default:** Off

#### `--profile` / `-p` `<string>`

Select a configuration profile defined in config.toml. (Also available as a global flag.)

- **Default:** No profile

#### `--sandbox` / `-s` `<read-only | workspace-write | danger-full-access>`

Sandbox policy for model-generated commands. (Also available as a global flag.)

- **Default:** Value from `config.toml`

#### `--config` / `-c` `<key=value>`

Inline configuration override for the non-interactive run (repeatable). (Also available as a global flag.)

- **Default:** Values from `config.toml`

#### Positional: `PROMPT`

Initial instruction for the task. Use `-` to pipe the prompt from stdin.

```bash
codex exec "fix the CI failure"
echo "explain auth" | codex exec -
```

#### `codex exec resume` switches

Resume a previous exec session by ID or `--last`.

| Switch | Type | Description |
|---|---|---|
| `--last` | flag | Resume the most recent conversation from the current working directory |
| `--all` | flag | Include sessions outside the current working directory when selecting the most recent session |
| `-i` / `--image` | `path[,path...]` | Attach one or more images to the follow-up prompt |
| Positional: `SESSION_ID` | `uuid` | Resume the specified session. Omit and use `--last` to continue the most recent session |
| Positional: `PROMPT` | `string` | Optional follow-up instruction sent immediately after resuming |

```bash
codex exec resume --last "fix the race conditions you found"
codex exec resume --all --last "continue from any directory"
codex exec resume 7f9f9a2e-1b3c-4c7a-9b0e-... "implement the plan"
```

### `codex resume` switches

Continue a previous interactive session.

| Switch | Type | Description |
|---|---|---|
| `--last` | flag | Skip the picker and resume the most recent conversation from the current working directory |
| `--all` | flag | Include sessions outside the current working directory |
| Positional: `SESSION_ID` | `uuid` | Resume the specified session |

```bash
codex resume                    # interactive picker
codex resume --last             # most recent in cwd
codex resume --all --last       # most recent anywhere
codex resume abc123-def456      # specific session
```

### `codex fork` switches

Fork a previous interactive session into a new thread, preserving the original transcript.

| Switch | Type | Description |
|---|---|---|
| `--last` | flag | Skip the picker and fork the most recent conversation |
| `--all` | flag | Show sessions beyond the current working directory |
| Positional: `SESSION_ID` | `uuid` | Fork the specified session |

```bash
codex fork                      # interactive picker
codex fork --last               # fork most recent
codex fork abc123-def456        # fork specific session
```

### `codex login` switches

Authenticate Codex using ChatGPT OAuth, device auth, or an API key piped over stdin.

| Switch | Description |
|---|---|
| `--device-auth` | Use OAuth device code flow instead of launching a browser window (useful for headless environments) |
| `--with-api-key` | Read an API key from stdin |

```bash
codex login                                # browser-based OAuth
codex login --device-auth                  # headless device code flow
printenv OPENAI_API_KEY | codex login --with-api-key
```

`codex login status` prints the active authentication mode and exits with `0` when logged in. No flags.

### `codex logout` switches

No flags. Removes saved credentials for both API key and ChatGPT authentication.

```bash
codex logout
```

### `codex apply` switches

Apply the most recent diff from a Codex Cloud task to your local repository.

| Switch | Type | Description |
|---|---|---|
| Positional: `TASK_ID` | `string` | Identifier of the Codex Cloud task whose diff should be applied |

```bash
codex apply abc123-task-id
codex a abc123-task-id       # alias
```

Exits non-zero if `git apply` fails (e.g., due to conflicts).

### `codex cloud` switches

#### `codex cloud` (interactive picker)

No flags. Opens an interactive picker to browse active or finished tasks.

#### `codex cloud exec`

Submit a new Codex Cloud task directly.

| Switch | Type | Description |
|---|---|---|
| `--env` | `ENV_ID` | Target Codex Cloud environment identifier (required). Use `codex cloud` to list options. |
| `--attempts` | `1-4` | Number of assistant attempts (best-of-N). Default: `1`. |
| Positional: `QUERY` | `string` | Task prompt. If omitted, Codex prompts interactively for details. |

```bash
codex cloud exec --env my-env "Summarize open bugs"
codex cloud exec --env my-env --attempts 3 "Optimize the query"
```

#### `codex cloud list`

List recent Codex Cloud tasks with optional filtering and pagination.

| Switch | Type | Description |
|---|---|---|
| `--env` | `ENV_ID` | Filter tasks by environment identifier |
| `--limit` | `1-20` | Maximum number of tasks to return. Default: server default. |
| `--cursor` | `string` | Pagination cursor returned by a previous request |
| `--json` | flag | Emit machine-readable JSON instead of plain text |

```bash
codex cloud list
codex cloud list --env my-env --limit 10 --json
```

### `codex app` switches

Launch Codex Desktop from the terminal on macOS or Windows.

| Switch | Type | Description |
|---|---|---|
| `--download-url` | `url` | Advanced override for the Codex desktop installer URL |
| Positional: `PATH` | `path` | Workspace path for Codex Desktop. On macOS, opens this path; on Windows, prints the path. |

```bash
codex app                    # launch desktop app
codex app ~/projects/myapp   # launch with workspace path
```

### `codex app-server` switches

Launch the Codex app server locally. Primarily for development and debugging.

| Switch | Type | Description |
|---|---|---|
| `--listen` | `stdio:// \| ws://IP:PORT` | Transport listener URL. Default: `stdio://`. Use `ws://IP:PORT` to expose a WebSocket endpoint for remote clients. |
| `--ws-auth` | `capability-token \| signed-bearer-token` | Authentication mode for WebSocket clients. If omitted, auth is disabled; non-local listeners warn during startup. |
| `--ws-token-file` | `absolute path` | File containing the shared capability token. Required with `--ws-auth capability-token`. |
| `--ws-shared-secret-file` | `absolute path` | File containing the HMAC shared secret for signed JWT bearer tokens. Required with `--ws-auth signed-bearer-token`. Secret must be at least 32 bytes. |
| `--ws-issuer` | `string` | Expected `iss` claim for signed bearer tokens. Requires `--ws-auth signed-bearer-token`. |
| `--ws-audience` | `string` | Expected `aud` claim for signed bearer tokens. Requires `--ws-auth signed-bearer-token`. |
| `--ws-max-clock-skew-seconds` | `number` | Clock skew allowance when validating signed bearer token `exp` and `nbf` claims. |

```bash
codex app-server --listen stdio://                                # JSONL over stdio
codex app-server --listen ws://127.0.0.1:4500                    # WebSocket, localhost
codex app-server --listen ws://0.0.0.0:4500 \
  --ws-auth capability-token --ws-token-file ~/.codex/token      # With auth
codex app-server --listen ws://0.0.0.0:4500 \
  --ws-auth signed-bearer-token --ws-shared-secret-file ~/.codex/secret \
  --ws-issuer my-issuer --ws-audience my-audience                # JWT auth
```

### `codex completion` switches

Generate shell completion scripts. Output prints to stdout.

| Switch | Type | Description |
|---|---|---|
| Positional: `SHELL` | `bash \| zsh \| fish \| power-shell \| elvish` | Shell to generate completions for |

```bash
codex completion bash
codex completion zsh
codex completion fish
codex completion power-shell
```

Install example for zsh:

```bash
eval "$(codex completion zsh)"
```

### `codex features` switches

Manage feature flags stored in `~/.codex/config.toml`.

| Subcommand | Usage | Description |
|---|---|---|
| `list` | `codex features list` | Show known feature flags, their maturity stage, and their effective state |
| `enable` | `codex features enable <feature>` | Persistently enable a feature flag. Respects the active `--profile` |
| `disable` | `codex features disable <feature>` | Persistently disable a feature flag. Respects the active `--profile` |

```bash
codex features list
codex features enable unified_exec
codex features disable shell_snapshot
```

### `codex mcp` switches

Manage Model Context Protocol servers.

#### `codex mcp add <name>`

| Switch | Type | Description |
|---|---|---|
| `--url` | `https://...` | Register a streamable HTTP server. Mutually exclusive with `COMMAND...`. |
| `--env` | `KEY=VALUE` | Environment variable assignments for a stdio server. Repeatable. |
| `--bearer-token-env-var` | `ENV_VAR` | Environment variable whose value is sent as a bearer token for HTTP servers |
| Positional: `COMMAND...` | args | Executable plus arguments to launch the MCP server (stdio transport). Provide after `--`. |

```bash
codex mcp add my-server -- npx my-mcp-server
codex mcp add my-server --url https://mcp.example.com
codex mcp add my-server --bearer-token-env-var MY_TOKEN --url https://mcp.example.com
```

#### `codex mcp get <name>`

| Switch | Type | Description |
|---|---|---|
| `--json` | flag | Print the raw config entry |

#### `codex mcp list`

| Switch | Type | Description |
|---|---|---|
| `--json` | flag | Machine-readable output |

#### `codex mcp login <name>`

| Switch | Type | Description |
|---|---|---|
| `--scopes` | `scope1,scope2` | Scopes requested during OAuth login for a streamable HTTP server |

#### `codex mcp logout <name>`

No flags. Removes stored OAuth credentials.

#### `codex mcp remove <name>`

No flags. Deletes a stored MCP server definition.

### `codex mcp-server` switches

No additional flags. Runs Codex as an MCP server over stdio. Inherits global configuration overrides. Exits when the downstream client closes the connection.

```bash
codex mcp-server
npx @modelcontextprotocol/inspector codex mcp-server
```

### `codex execpolicy` switches

Evaluate execpolicy rule files and see whether a command would be allowed, prompted, or blocked.

#### `codex execpolicy check`

| Switch | Type | Description |
|---|---|---|
| `--pretty` | flag | Pretty-print the JSON result |
| `-r` / `--rules` | `path` (repeatable) | Path to an execpolicy rule file. Provide multiple flags to combine rules. |
| Positional: `COMMAND...` | args | Command to be checked against the specified policies |

```bash
codex execpolicy check --pretty --rules ./my.rules -- rm -rf /
codex execpolicy check -r base.rules -r extra.rules -- npm install
```

### `codex sandbox` switches

Run arbitrary commands inside Codex-provided sandboxes.

#### `codex sandbox macos` (alias: `codex debug seatbelt`)

| Switch | Type | Description |
|---|---|---|
| `-c` / `--config` | `key=value` | Configuration overrides (repeatable) |
| `--full-auto` | flag | Grant write access to workspace and `/tmp` without approvals |
| `--log-denials` | flag | Log denied sandbox operations |
| Positional: `COMMAND...` | args | Command to execute under macOS Seatbelt |

```bash
codex sandbox macos --full-auto -- python script.py
```

#### `codex sandbox linux` (alias: `codex debug landlock`)

| Switch | Type | Description |
|---|---|---|
| `-c` / `--config` | `key=value` | Configuration overrides (repeatable) |
| `--full-auto` | flag | Grant write access to workspace and `/tmp` |
| Positional: `COMMAND...` | args | Command to execute under Landlock+seccomp |

```bash
codex sandbox linux --full-auto -- python script.py
```

#### `codex sandbox windows`

| Switch | Type | Description |
|---|---|---|
| `--full-auto` | flag | Grant write access to workspace and temp directories |
| Positional: `COMMAND...` | args | Command to execute under Windows restricted token |

```bash
codex sandbox windows --full-auto -- python script.py
```

### `codex plugin marketplace` switches

Manage plugin marketplaces from Git or local sources.

#### `codex plugin marketplace add <source>`

Source can be: GitHub shorthand (`owner/repo` or `owner/repo@ref`), HTTP/HTTPS Git URL, SSH Git URL, or a local directory.

| Switch | Type | Description |
|---|---|---|
| `--ref` | `REF` | Pin a specific Git ref |
| `--sparse` | `PATH` | Sparse checkout path (repeatable, Git sources only) |

```bash
codex plugin marketplace add my-org/plugins
codex plugin marketplace add my-org/plugins --ref v1.0 --sparse path/to/dir
```

#### `codex plugin marketplace remove <marketplace-name>`

No flags. Removes a configured marketplace.

#### `codex plugin marketplace upgrade [marketplace-name]`

No flags. Refreshes one configured Git marketplace, or all when no name is provided.

### `codex debug app-server send-message-v2` switches

Send one message through app-server's V2 thread/turn flow.

| Switch | Type | Description |
|---|---|---|
| Positional: `USER_MESSAGE` | `string` | Message text to send through the built-in V2 test-client flow |

```bash
codex debug app-server send-message-v2 "Hello, test message"
```

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
- [Codex Remote Connections](https://developers.openai.com/codex/remote-connections/)
