---
hash: ef46db3751d8e999-6bf9593d6502cd51
last_updated: 2026-07-14
---
# Claudine CLI Reference

Complete command documentation with examples and options.

## Global Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Increase presentation detail for human-facing output (repeatable) |
| `--debug <LEVEL>` | Set diagnostic tracing level (`trace`/`debug`/`info`/`warn`/`error`) |
| `--plain` | Strip ANSI escape codes from all output |
| `--version` | Print version information |
| `-h, --help` | Print help information |

`-v`/`--verbose` controls styled user-facing output; `--debug` (or `RUST_LOG`) controls raw tracing — they are separate surfaces. Running `claudine` with no subcommand renders rich grouped help (the retired `about` command).

---

## `claudine init`

Interactive setup wizard for initial configuration. Walks through 4 phases:

1. **Agent Discovery** — detects installed agentic CLIs on the system
2. **Provider Preferences** — rank your favorite installed CLIs for canonical ordering
3. **Action Defaults** — global interview (logging `all/some/none`, then input-needed actions)
4. **Write & Register** — saves `~/.claudine/config.json` and registers hooks with each provider

Setup automatically configures all detected available agents (no per-agent selection prompt). Claudine auto-configures every event each provider supports via native hooks. Events with no actions are still registered as explicit no-op bindings.

```bash
claudine init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--quick` | Use defaults without prompting |
| `--repo` | Create project-scoped configuration |

**Quick Mode:**

```bash
claudine init --quick
```

Creates configuration with sensible defaults:
- All detected agents enabled
- `session_start` → SoundEffect (power-up)
- `turn_complete` → SoundEffect (success)
- `tool_error` → SoundEffect (error)
- `permission_request` → SoundEffect (notification)
- `human_in_the_loop` → SoundEffect (notification)

`--repo` creates `.claudine/config.json` in the repository root and can add `.claudine/` to `.gitignore`.

---

## `claudine hooks`

Inspect hook registrations and provider capabilities.

```bash
claudine hooks [OPTIONS] [PROVIDER]
```

| Option | Description |
|--------|-------------|
| *(none)* | Status report: install state, registration drift per provider, legend, protect status, `claudine sync --fix` hint when drift exists |
| `-v` | Adds action count indicators per event |
| `<provider>` | Detailed event/action view for one provider (fuzzy matching), including per-event capture method and its unmapped native events |
| `--support` | Event support glyph matrix across all providers (✅ hook / 🔶 indirect / 🅐 acp / – none) |
| `--capture-method` | Hidden alias of `--support` (its per-level detail moved to the provider detail view) |
| `--mapping` | Native event name mappings per provider |
| `--describe` | Event descriptions, payload schemas, and return schemas |
| `--variables` | All 28 template variables with current detected values |

**Provider fuzzy matching**: commands that accept a provider name use a 3-tier resolution: exact match → prefix match → contains match. This lets users type `cl` instead of `claude`.

**Sound effect validation**: runs automatically when viewing hooks and uses a 5-tier fuzzy matching algorithm (exact, normalized, prefix, contains, Levenshtein-like) to suggest replacements for invalid effect names.

**Basic output:**

```
Provider    Installed  Subscribed Hooks
Claude      ✓          session_start, turn_complete, tool_error
Codex       ✓          turn_complete
Gemini      ✓          -
OpenCode    ✗          -
```

Event names in the Subscribed Hooks column are color coded, and the legend below the table documents the coding: yellow = missing (not yet registered), red = stale (registered but no longer configured), red strikethrough = unsupported (won't fire). When any drift exists the report closes with a hint to run `claudine sync --fix`.

**Provider detail view:**

```bash
claudine hooks claude
```

Shows detailed event/action configuration for a specific provider. The Capture column carries the per-event capture method — `hook`, `stream-parse (protocol)`, `wire-proxy (mode)`, `wrapper`, or `acp`. Providers with native hook phases Claudine's 16-event model cannot represent (e.g. Gemini `BeforeToolSelection`, OpenCode `tool.definition`) get a closing "Not mappable — configure natively" section with remediation guidance.

**Support matrix:**

```bash
claudine hooks --support
```

Shows which events each provider supports as a glyph matrix. One glyph vocabulary drives both cells and legend:
- ✅ = hook (config-file registration)
- 🔶 = indirect (delivered via wrapper, stream parsing, or wire proxy)
- 🅐 = acp (captured via the Agent Client Protocol)
- – = not supported (no capture path)

When the terminal is too narrow for all seven provider columns, the matrix degrades into stacked tables of fewer providers (it never refuses to render). `--support` and `--mapping` both close with the "Not mappable — configure natively" list of provider-native events with no canonical mapping.

---

## `claudine skills` · `claudine commands` · `claudine agents`

List shared resources across providers and show their link/sync state. Each command targets one resource type (the former unified `link` command was split into these three):

| Command | Resource |
|---------|----------|
| `claudine skills` | Agentic skills |
| `claudine commands` | Slash commands |
| `claudine agents` | Agent/subagent definitions |

Each accepts an optional `[provider]` (fuzzy-matched) for a detailed per-provider view and shows link state across providers.

**Behavior:**

- Inside git repo: links repo-scoped resources using **relative** symlinks
- Outside git repo: links user-scoped resources using **absolute** symlinks

**Example output:**

```
Linked:
  ✓ clap         Claude → Codex, Gemini
  ✓ tokio        Claude → Codex, Gemini

Already in sync:
  = axum         Claude ↔ OpenCode (identical content)

Skipped:
  ~ chrono       Claude → OpenCode (OpenCode reads .claude/skills/)

Conflicts:
  ✗ react        Claude (a1b2c3) ≠ Codex (e5f6g7)
```

---

## `claudine actions`

Show which actions are configured and for which events, across providers.

```bash
claudine actions
```

---

## `claudine config`

Manage Claudine configuration through a TUI. The current TUI focus is messenger routes — bot-token routes (Discord, Slack, Signal, WhatsApp) and webhook routes (Discord/Slack webhooks).

```bash
claudine config
```

- Webhook URL fields use masked input and are validated before advancing; env-only routes (blank URL + non-empty env var) are allowed.
- Inline webhook URLs never render raw (shown as `webhook: ********`); all webhook send errors run through `redact_webhook_urls`.
- A **Test Connection** workflow (press `T` during webhook input) sends a test message without saving the route.
- Desktop notifications are intentionally absent — they are zero-config and triggered via lifecycle `notify` frontmatter only.

---

## `claudine providers`

Show a compact provider capability matrix with provider name as an OSC8 link to provider documentation, plus Skill, Slash, Agent, and Hooks columns.

```bash
claudine providers
```

**`claudine providers generate [slug]`** shells out to the `claudine-gen` binary (the CLI never links the generator). Default: forwards to `claudine-gen generate` with inherited stdio — per-file diff + `[y/N/q]` confirmation on a TTY, report-only otherwise; declined drift prints the field-keyed override snippet and exits non-zero.

| Option | Description |
|--------|-------------|
| `--dry-run` | Report-only regardless of TTY (forwarded) |
| `--yes` | Write every drifted file without prompting (forwarded) |
| `--mapping` | Render the field → source → coercion mapping registry as a table |
| `--mapping --json` | Raw mapping JSON pass-through |

**`claudine providers agent-errors check <slug> [--findings <path>]`** runs the deterministic `agent-errors` research gate through the same generator-binary boundary. The command writes an explicit `clean`, `findings`, or `gate_error` outcome report and is safe to reference from a lifecycle shell action; the shell policy continues to blacklist direct `cargo` invocations.

---

## `claudine sync`

Re-apply hook registrations to match the current config.

```bash
claudine sync [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would change without writing |
| `--provider <name>` | Only sync specific provider |
| `--fix` | Remove unsupported events from config |

**Use cases:**
- After manually editing `~/.claudine/config.json`
- After updating `claudine` binary location
- To restore hooks after agent config reset
- With `--fix`: clean up events that don't work with certain providers

**Fix mode:**

When `claudine sync` warns about unsupported events:

```
⚠ Warning: Some configured events are not supported by their providers:
  Codex: tool_error, subagent_stop
  OpenCode: subagent_stop
```

Use `--fix` to automatically remove them:

```bash
claudine sync --fix
```

Preview what would be removed:

```bash
claudine sync --fix --dry-run
```

---

## `claudine handle <event>`

Process an incoming event from a provider hook. Reads JSON payload from stdin, auto-detects the provider from payload structure (or accepts `--provider` override), resolves environment context, and dispatches through the event pipeline.

```bash
claudine handle <EVENT> [OPTIONS]
```

**Execution Deadline.** To prevent hook handlers from blocking the parent agent session, `claudine handle` enforces a hard **5-second deadline** by default (overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`). When exceeded, the handler aborts with a diagnostic message to stderr and exits 124. Individual bash and messenger actions also have tighter 3s timeouts when running inside a hook handler.

| Option | Description |
|--------|-------------|
| `--provider <name>` | Provider hint (auto-detected from payload) |

**Input:** JSON event payload via stdin

**Output:** JSON response to stdout (if provider expects it)

**Exit codes:** `0` = success, `2` = block (if supported), `124` = deadline exceeded

**Stdin auto-detection**: provider is detected from JSON payload structure (`hook_event_name` → Claude, `type` + `thread_id` → Codex, etc.) so hooks don't need to pass `--provider` explicitly.

```bash
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash"}' | claudine handle before_tool
```

---

## Composition Commands

Markdown frontmatter-based composition pipelines for delivering prompts to provider sessions. All three commands reuse the wrapper pipeline (env setup, harness detection, structured streaming, lifecycle-stack recovery).

### `claudine compose <file-ref> [key=value ...]`

Compose a Markdown file and send the result as a prompt. No file mutation.

```bash
claudine compose @prompts/review.md review=review.md
```

### `claudine inline-compose <file-ref> [key=value ...]`

Use frontmatter `prompt` to generate content and replace the document body. Preserves frontmatter, updates `last_updated`.

```bash
claudine inline-compose @notes/update.md draft=false
```

### `claudine sequence <file-ref> [key=value ...]`

Run a serial sequence of composition steps declared in a single document. Shared shell approval cache across steps.

```bash
claudine sequence @research.md topic="async traits"
```

### `--dry-run`

Runs the full composition pipeline **up to but not including provider launch**, then emits the composed result instead of sending it to a provider. Available on all three commands; suitable for CI rehearsal because it exercises the identical path minus the spawn.

- Schema validation, shell-command execution (real side effects), and the shell-audit pre-flight all run normally.
- The provider is never launched; `inline-compose --dry-run` therefore **does not mutate** the source file.
- **stdout** = composed body; **stderr** = highlighted YAML frontmatter + a metadata table (Document as a blue OSC8 link, Description, Agent, Model, YOLO, Session mode/source, and Area when inside a monorepo). So `compose --dry-run doc.md > body.md` captures only the body.
- `--quiet` / `--silent` have **no effect** under `--dry-run`.
- **Non-TTY shell gate:** an unapproved shell command in a non-TTY environment exits non-zero with `Cannot dry-run: shell command 'X' requires interactive approval. Run with --yolo to auto-approve, or pre-approve the command in your configuration.` In a TTY the normal interactive approval prompt fires. Bypass with `--yolo`.
- **`sequence --dry-run`** concatenates all step bodies to stdout in order, prints each step's metadata to stderr separated by a `=== Document N of M ===` divider (before every document after the first), and fails fast on the first composition error.

See [Composition — Dry Run](composition.md#dry-run) for the full reference.

### Session interactivity

Composition commands resolve session interactivity from (highest to lowest precedence):

1. `--no-interactive` CLI flag
2. `-i` / `--interactive` CLI flag
3. `interactive` frontmatter property (`true` / `false`)
4. Default: non-interactive

`--interactive` and `--no-interactive` are mutually exclusive; clap rejects `-i --no-interactive` at parse time. The `interactive` frontmatter property is honored by `compose` and `inline-compose`; `claudine sequence` rejects `interactive: true` because a sequence is serial automation and must be driven by the explicit `--interactive` override when needed.

### `--perf`

Opt-in flag (composition commands and the provider wrappers) that prints a **reconciling performance tree** to **stderr** after the run. The `Performance` headline is true wall-clock and equals the sum of its top-level `Structural` buckets (`pre-dispatch`, `prep phase`, `environment setup`, `agent execution`) plus a synthetic `unattributed` remainder — the headline can never contradict the body. Nested `Breakdown` rows itemize cost (Darkmatter composition stages, agent sub-timings) without double-counting; a percent column shows each row's share of wall-clock, a single `▇ HOT` marker flags the dominant leaf (≥20% of wall-clock), and `×N` annotates stages that ran more than once. Dry runs render `agent execution` as an `—` leaf. The report is stderr-only (never pollutes piped stdout) and is emitted even under `--silent`/`--quiet`. `sequence` aggregates one report across all steps. See [Composition — Performance Reporting](composition.md#performance-reporting) for the full reference.

**Positional Arguments:**
- Exactly one file reference (supports `@` magic paths)
- Zero or more `key=value` setters (overrides frontmatter)

**Common Flags:**
- `--claude`, `--codex`, `--gemini`, `--opencode`, etc.
- `-i, --interactive`
- `--no-interactive`
- `-m, --model <MODEL>`
- `-s, --system-prompt <PROMPT|FILE>`
- `-t, --timeout <DURATION>`
- `--stall-timeout <DURATION>` (OpenCode-only stalled-generation backstop; `0s` disables; env `CLAUDINE_OPENCODE_STALL_TIMEOUT`, frontmatter `stall_timeout`, built-in `10m`)
- `--dry-run`, `-q, --quiet`, `--silent`, `--perf`

---

## `claudine logs`

Query the local reporting index built from JSONL hook logs.

```bash
claudine logs [SUBCOMMAND] [FLAGS]
```

Shared filters: `--provider`, `--repo`, `--package-area`, `--package`. Read commands perform a best-effort sync before querying. Time-window commands also accept nested error drill-downs such as `claudine logs week errors` and `claudine logs today errors`.

| Subcommand | Description |
|------------|-------------|
| `today` | Today's session summary |
| `week` | This week's summary |
| `month` | This month's summary |
| `sessions` | List recent sessions |
| `tools` | Tool usage breakdown |
| `errors` | Error log |
| `repos` | Per-repo summary |
| `trends` | Usage trends over time |
| `drift` | Model-catalog drift signals and family-latest alias resolutions |
| `sync` | Force re-sync of JSONL logs into SQLite |

---

## `claudine dashboard`

The mesh NOW view — a one-shot, read-only render over the rendezvous daemon's live registers (rendezvous dashboard feature, D1). The historical complement is `claudine logs`; this command answers "what is running right now, where, and does anything need me?"

```bash
claudine dashboard [--local]
```

- **Mesh-wide by default** — renders every host the local daemon holds a register replica for. `--local` restricts to this host only.
- **Data path** — folds five daemon read RPCs into one view: `ListActiveSessions` (live sessions), `ListHostCapabilities` (hostname/OS/arch/CPU/RAM/GPU), `ListHostRepos` (checked-out repo count), `ListPeers` (per-peer `last_synced_unix_ms`, the freshness clock), and `Status` (the local node id, to mark the always-fresh local host).
- **Staleness (D4)** — every remote host row shows its last-sync age. Past **60 seconds** of sync silence a host is rendered *stale* and its sessions as *unknown* rather than as last-known status; a host never synced renders *never synced*. The clock is the daemon's `last_synced_unix_ms` (stamped only on a successful direct-sync round — not mDNS chatter), kept advancing by the daemon's periodic re-sync worker.
- **Needs human intervention (D5, triggers 1 & 2)** — session status is a **typed per-producer model with a daemon-side precedence reducer** (session-state foundation, 2026-07-13), not a bare last-writer-wins string. Each producer owns one `status_slots` entry; the daemon folds them by intervention strength (`waiting_on_user` > `idle` > `active`, ties by revision) and projects three fields the dashboard reads: the backward-compatible flat `status`, plus `status_basis` (why) and `status_producer` (who). Trigger 1 is **hook-primary**: the permission signal is reported from the normalized `claudine handle` hook boundary so it covers interactive PTY sessions (which run no stream sink), with the wrapper stream sink demoted to a stamped fallback. The `Needs?` column is **tiered and honest**: (a) a fresh `waiting_on_user` renders a strong "⚠ input" badge (carrying its basis/producer provenance and reported age); (b) a fresh `idle` (Trigger 2, below) renders a weaker dim "◦ idle" badge; (c) a `permission_signal:"supported"` provider with nothing outstanding renders "no intervention needed"; (d) a `permission_signal:"unsupported"` provider (no permission-signal capability, recorded at STARTED from the `claudine hooks --support` matrix) renders "permission signal unavailable" — so absence of a signal reads as "can't tell", never mislabeled as fine. Untrusted (stale) hosts suppress all of these to "—".
- **Interactive-idle signal (D5, trigger 2, IMPLEMENTED 2026-07-13)** — a wrapped **interactive** session that has been idle since its last assistant turn completed is the agent waiting on the user. It is a **hook-driven producer** (not the stream sink, which the interactive PTY path never builds): the wrapper injects `CLAUDINE_INTERACTIVE=1` into the child env, and `claudine handle` reports `idle` on a turn-complete event (Claude's `Stop` → `AgenticEvent::TurnComplete`) and clears back to `active` on the next user prompt (`AgenticEvent::BeforePrompt`). Non-interactive turn-completes report nothing (the agent auto-proceeding is not a human wait). This weaker `idle` writes its own `IdleHook` reducer slot (basis `interactive_turn_complete`), so it can **never** clobber an unresolved stronger `waiting_on_user`. The badge shows the idle duration ("◦ idle 45s") for **local** sessions only — a remote session's `updated_at` is stamped by the remote daemon, so its age is meaningless under clock skew and renders "◦ idle" without a duration. The heading appends an idle count when non-zero.
- **v1 scope** — wrapped sessions only; unwrapped sessions appear once the process monitor lands. An absent daemon degrades to a friendly note (exit 0), never an error.
- **Dual-target** — rendered through `DashboardReport` (`TerminalRenderable` + `BrowserRenderable`, the `MetricsReport` precedent); the component lives CLI-local (`cli/src/commands/dashboard/`) so the `claudine` library stays free of any `rendezvous-*` dependency.

---

## `claudine mcp`

Manage Claudine's normalized MCP catalog and provider sync state.

```bash
claudine mcp [SUBCOMMAND] [--json]
```

| Subcommand | Description |
|------------|-------------|
| *(none)* | List catalog entries, defaults, and provider presence |
| `init` | Import supported native provider MCP configs into `~/.claudine/mcp/` |
| `show <id>` | Show normalized definition and provenance for one server |
| `default [ids...]` | Replace user-scope default server IDs |
| `default --repo [ids...]` | Replace repo-scope default server IDs |
| `alias add <id> <alias>` | Add a catalog alias |
| `alias remove <alias>` | Remove an alias |
| `remove <id>` | Remove a catalog entry after confirmation |
| `sync <provider> [--scope user\|repo] [--apply]` | Dry-run or apply export of effective defaults to a provider's native config |

Storage lives in `~/.claudine/mcp/catalog.json`, `~/.claudine/mcp/defaults.json`, `~/.claudine/mcp/provider-state.json`, and optional repo defaults at `<repo>/.claudine/mcp.json`. Repo defaults replace user defaults.

---

## Wrapped Provider Commands

Claudine can wrap provider CLIs with preflight checks, argument translation, environment sanitization, and structured streaming:

- `claudine claude`
- `claudine codex`
- `claudine gemini`
- `claudine kimi`
- `claudine qwen`
- `claudine opencode`
- `claudine goose`

### Shared Wrapper Flags

| Flag | Description |
|------|-------------|
| `-y, --yolo` | Translate to provider-specific auto-approval mode (OpenCode: non-interactive only — pushes `--dangerously-skip-permissions` **and** merges a session-wide `permission` block into `OPENCODE_CONFIG_CONTENT` so subagents are also auto-approved; warn-only/ignored in OpenCode interactive sessions) |
| `-i, --interactive` | Force interactive mode even when a prompt string is provided |
| `-m, --model <MODEL>` | Override the model used by the provider |
| `--asp <FILE>` | Append a system prompt from a file (alias: `--append-system-prompt`) |
| `--rsp <FILE>` | Replace the provider's system prompt with contents from a file (alias: `--replace-system-prompt`) |
| `-t, --timeout <DURATION>` | Wall-clock timeout like 30s, 5m, 2h (non-interactive only) |
| `--stall-timeout <DURATION>` | OpenCode-only stalled-generation backstop (live-but-dead retry-churn guard); built-in `10m`, `0s` disables. Inert config on non-OpenCode providers. See [Timeouts](timeouts.md#opencode-stalled-generation-backstop) |
| `-o, --output <FORMAT>` | Set output format (json, text, stream) |
| `--include <ENV_NAME>` | Keep a sensitive env var name that would otherwise be filtered |
| `--mcp` | Compose a Claudine-managed MCP session from the effective defaults |
| `--use <ID[,ID...]>` | Add specific MCP catalog IDs or aliases and enable MCP composition |
| `--sandbox` | Enable provider-specific sandboxing |
| `--repo` | Use only repo-scoped skills, commands, and agents via a shadow HOME |
| `-p, --prompt-file <FILE>` | Source initial prompt from a Markdown file (composed with Darkmatter) |
| `--frontmatter-prompt <FILE>` | Inline composition: use frontmatter prompt as input |
| `--compose <FILE>` | Chained composition: compose full document and use as prompt |
| `--dry-run` | Show what would be executed without launching the child |
| `-q, --quiet` | Show only the header line; suppress env details |
| `--silent` | Suppress all Claudine preflight output |
| `--perf` | Print a reconciling performance tree to stderr after the run (see [`--perf`](#--perf)) |
| `-- ...` | Force all remaining args to passthrough unchanged |

### Wrapper Behavior

- **Interactivity default**: providing a prompt string implies non-interactive mode. Use `-i`/`--interactive` to override back to interactive when providing a startup prompt.
- **Execution line**: displays `Claudine ▸ {provider} {badges} {prompt}` — only the user's prompt text is shown (provider-specific switches are not leaked). Truncated to one terminal line.
- **Structured streaming**: non-interactive runs use provider-native structured output (stream-json, JSONL, NDJSON) as the internal control plane. Claudine parses the stream live, reconstructs clean assistant text for stdout, and emits metadata summaries to stderr.
- **Stderr summaries**: session-start info (session ID, model), completion summary (duration, tokens, cost, tool calls), and verbose details (tools used, turns, stop reason).
- **Verbosity**: `--quiet` shows only a compact completion line; `--silent` suppresses all Claudine output; `-v` adds detailed metadata on the second summary line.
- Validates provider binary availability before spawn (with provider docs URL in errors).
- Filters sensitive env vars whose names contain `API_KEY` or `TOKEN` unless explicitly included.
- Reports removed env variable names to stderr (names only, sorted/unique).
- Injects `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, `CLAUDINE_PID`, and, when resolvable in monorepos, `PACKAGE_AREA` and `PACKAGE`.
- `--mcp` resolves repo defaults if `<repo>/.claudine/mcp.json` exists, otherwise user defaults; `--use` appends explicit IDs or aliases and also enables MCP mode.
- Non-interactive Codex, Gemini, and OpenCode runs also strip catalog-resolvable `#tags` from the prompt and activate the matching servers.
- Writes a synthetic JSONL summary event per session for reporting completeness.

---

## `claudine completions <shell>`

Generate shell completion scripts.

```bash
claudine completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`

```bash
# Bash
claudine completions bash > ~/.local/share/bash-completion/completions/claudine

# Zsh
claudine completions zsh > ~/.zfunc/_claudine

# Fish
claudine completions fish > ~/.config/fish/completions/claudine.fish
```

**Related runtime behavior.** The `completions` command only installs the
static shell script. Runtime file selection also happens through the
ENTER-path autocomplete: when a composition command runs interactively
and a required file value is missing (omitted positional argument or a
`file`/`file[]` schema property), Claudine opens a `ChooseOne` or
`ChooseMany` chooser. See [Shell Completions](completions/shell-completions.md)
for details.

---

## `claudine context`

Show Darkmatter's runtime context variables, expression engine, and side-effect capabilities.

```bash
claudine context [OPTIONS]
```

| Option | Description |
|--------|-------------|
| *(none)* | Display the complete context-variable catalog |
| `--values` | Display live captured values for each variable |
| `--expressions` | Display the expression engine's operators and functions |
| `--side-effects` | Display the side-effect capability catalog |

The flags `--values`, `--expressions`, and `--side-effects` are mutually exclusive.

### Default report

Lists every context variable exposed by Darkmatter, grouped by category and subsection:

| Column | Content |
|--------|---------|
| `Property` | Canonical variable name prefixed with `ctx.` |
| `Type` | Display type (`String`, `DateTime`, `Boolean`, etc.) |
| `Description` | Short description of the variable |

Context variable categories include date/time, repository metadata, file changes, languages, documents, operating system, and hardware. The `Property` and `Type` column widths are computed once across the entire catalog and reused for every section.

### `--values` report

Same sections and column layout as the default report, but replaces `Description` with `Value`. Values are captured once per invocation through Darkmatter's runtime context API. Missing or unavailable values render as a dimmed `null` rather than being dropped.

Value formatting:
- strings render as their raw value
- booleans and numbers render textually
- arrays render as comma-separated items
- objects render as compact serialized JSON
- null or unavailable values render as `null`

### `--expressions` report

Displays the expression-language overview followed by the complete typed function catalog.

The overview covers operator precedence, truthiness rules, unary/comparison/arithmetic operators, variable access syntax (`ctx.today`, `env.HOME`, dot and bracket forms), the two parser modes (interpolation vs. condition), and null propagation behavior.

The function catalog is grouped by category (Type Predicates, Math, Collection, String Predicates, String Mutations, Date Formatting, Date Validators, Logical, Type Conversion, Filesystem) with columns:

| Column | Content |
|--------|---------|
| `Function` | Canonical snake_case signature |
| `Description` | Behavior and return-value summary |

### `--side-effects` report

Displays Darkmatter's complete side-effect capability catalog. This is documentation-only — no capabilities are invoked, probed, or checked for availability.

Capabilities are grouped by category (Frontmatter Mutations, File & Directory, Network) with columns:

| Column | Content |
|--------|---------|
| `Capability` | Canonical signature, including all overloaded arities |
| `Description` | Behavior and return-value summary |
| `Safety` | Applicable constraint (`FilesystemWrite`, `Network`, `MarkdownMutation`) |

Catalog-wide constraints communicated in the report:
- the report is documentation only and does not invoke side effects
- only an external orchestrator invokes side effects
- filesystem writes are restricted to the configured mutation root
- network operations are restricted by the deny-all-by-default host allowlist
- Markdown mutations honor Darkmatter's auto-rehash behavior
- catalog membership does not imply authorization or availability

### Rendering contract

All reports share a consistent terminal layout:
- Tables fill the available terminal width up to a maximum of **140 visible cells** total, including 1ch left and right margins, borders, separators, and content
- At widths below 140ch, tables use the available width without intentional overflow
- The **minimum supported terminal width is 53 visible cells**. At or above 53 cells every report renders all of its required columns by wrapping — no column is dropped and no Claudine-specific narrow layout is introduced. (The `--expressions` and `--side-effects` reports hold narrower unbreakable tokens and keep all columns well below 53; 53 is the binding floor for the default and `--values` reports.)
- Below 53 cells the terminal is unsupported: the shared `Table` component's own constrained-width behavior applies and it may emit its width-error diagnostic instead of a table. Widen the terminal to 53 or more cells to restore full rendering.
- Backtick-delimited inline code renders with inverse styling in styled output and visible backticks in plain/`--plain`/`NO_COLOR` mode
- Unordered lists use `- ` bullets with hanging indentation for wrapped continuation lines

---

## `claudine uninstall`

Remove hook registrations from all detected agents.

```bash
claudine uninstall [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--keep-config` | Keep `~/.claudine/config.json` (only remove hooks) |

**What it does:**
1. Deregisters hooks from all agent configs
2. Removes backup directory (`~/.claudine/backups/`)
3. Optionally removes `~/.claudine/config.json`

---

## CLI Module Structure

```
cli/src/
├── main.rs              → Entry point, clap parsing, command dispatch
├── args.rs              → CLI/Commands clap definitions
├── log.rs               → Output formatting (message/data/info/warn/error)
├── output.rs            → Execution line, badges, env details, prompt display
└── commands/
    ├── actions.rs       → Configured-action inspection
    ├── agents.rs        → Agent/subagent listing and link state
    ├── completions.rs   → Shell completion generation (+ hidden __complete engine)
    ├── compose.rs       → compose / inline-compose
    ├── config_tui/      → Configuration TUI (messenger routes)
    ├── handle.rs        → Event processing from stdin
    ├── help.rs          → Rich grouped help (no-subcommand entry)
    ├── hooks/           → Hook inspection and validation
    ├── init/            → Setup wizard orchestration
    ├── init_wizard.rs   → Wizard flow
    ├── link_display.rs  → Shared link-state rendering
    ├── logs/            → JSONL log reporting queries
    ├── mcp/             → MCP catalog, defaults, aliasing, import, and sync commands
    ├── providers.rs     → Provider capability matrix (skill/slash/agent/hooks)
    ├── sequence.rs      → Serial composition sequence
    ├── skills.rs        → Skill listing and link state
    ├── slash_commands.rs → Slash command listing and link state
    ├── sync.rs          → Hook re-registration
    ├── uninstall.rs     → Hook removal
    └── wrap/            → Shared wrapper pipeline, env sanitization, exec, stream capture, MCP injection
```

## Output System

All user-facing output goes through `log.rs`:

| Function | Target | Purpose |
|----------|--------|---------|
| `message()` | stderr | Always visible (status messages) |
| `data()` | stdout | Pipeable data output |
| `output()` | stdout | Inline output (no trailing newline) |
| `info()` | stderr | Only when verbosity enabled |
| `warn()` | stderr | Yellow "warning:" prefix |
| `error()` | stderr | Red "Error:" prefix (with leading blank line) |

Rich formatting uses biscuit-terminal components (Table, Prose with `{{bold}}` / `{{cyan}}` / `{{dim}}` markup, UnorderedList, OSC8 hyperlinks).

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Block action (when called as hook) |
| `3` | Configuration error |
| `4` | Provider not found |
| `5` | Permission denied |

---

## File Locations

| File | Path |
|------|------|
| User config | `~/.claudine/config.json` |
| Repo config | `<repo>/.claudine/config.json` |
| MCP catalog | `~/.claudine/mcp/catalog.json` |
| MCP defaults | `~/.claudine/mcp/defaults.json` |
| MCP state | `~/.claudine/mcp/provider-state.json` |
| Repo MCP defaults | `<repo>/.claudine/mcp.json` |
| Backups | `~/.claudine/backups/<provider>/<timestamp>.bak` |
| Event logs | `~/.claudine/logs/` (JSONL, daily rotation) |
| Reporting DB | `~/.claudine/logs/metrics.db` |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Diagnostic tracing level (also set via the `--debug <LEVEL>` flag) |
| `CLAUDINE_OPENCODE_STALL_TIMEOUT` | OpenCode stalled-generation backstop default (duration string; `0s` disables). Overridden by `--stall-timeout` / frontmatter `stall_timeout`; built-in `10m` |
| `HOME` | Used for path resolution |
| `PATH` | Must include `claudine` binary |
| `AGENT` | Injected by wrapper: provider name |
| `YOLO` | Injected by wrapper: auto-approval mode |
| `INTERACTIVE` | Injected by wrapper: interactivity flag |
| `AGENT_PARAMS` | Injected by wrapper: provider-specific args |
| `CLAUDINE_SESSION_ID` | Injected by wrapper: session identifier |
| `CLAUDINE_PID` | Injected by wrapper: Claudine's own process ID |
| `PACKAGE_AREA` | Injected by wrapper: monorepo package area (when resolvable) |
| `PACKAGE` | Injected by wrapper: monorepo package (when resolvable) |