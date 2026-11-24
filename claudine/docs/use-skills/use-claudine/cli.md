# The `claudine` CLI

```text
claudine <command> [params] [switches]
```

- **Provider names** are fuzzy-matched: an exact name, then a prefix, then a
  substring. So `cl` means `claude`, and `oc` or `open` means `opencode`.
- **Global options:** `-v` (more detail), `--debug <level>`, and `--plain`
  (no ANSI codes). `-V` prints the version.

## Install and first run

```bash
cd claudine && just install        # in the rusty-biscuit repo; installs `claudine`
claudine completions zsh > "${fpath[1]}/_claudine"   # bash | zsh | fish | powershell | elvish
```

There is no `init` command. The first command that needs configuration runs
setup: it's interactive on a terminal and writes defaults otherwise. The
result is `~/.claudine/config.yaml`. Change it later with `claudine config`
(a TUI), or with `claudine config set favorite-agent <slug|none>` and
`claudine config set prompt-for-missing <bool>`. Completion scripts call
back into `claudine`, so regenerate them only after an upgrade.

## Wrapped execution

```bash
claudine <provider> [switches] ['prompt'] [-- <args for the provider>]
```

Providers: `claude`, `codex`, `gemini`, `opencode`, `kimi`, `qwen`, `goose`,
`kilo`, `pi`, `antigravity` (runs the `agy` CLI).

Switches may go before or after the prompt; order never changes meaning.
Everything after the first `--` is passed to the provider unchanged.

| Switch | Effect |
|---|---|
| `-y, --yolo` | provider-specific auto-approve. `CLAUDINE_YOLO=1` does the same |
| `-i, --interactive` | interactive, even with a prompt (the prompt becomes the first turn) |
| `--edit` | write the prompt in `$VISUAL`/`$EDITOR`; an empty buffer quits. `--edit -i` makes it the first interactive turn |
| `-m, --model <M>` | model override |
| `--asp <FILE>` / `--rsp <FILE>` | append to, or replace, the provider's system prompt (long forms: `--append-system-prompt`, `--replace-system-prompt`) |
| `-t, --timeout <D>` | wall-clock limit such as `30s`, `5m`, or `2h` (non-interactive only) |
| `--step-timeout <D>` | kill after this long with no stream event (non-interactive only) |
| `--stall-timeout <D>` | OpenCode stalled-generation backstop (default `10m`; `0s` disables) |
| `--include <ENV>` | keep an env var the secret filter would remove (repeatable) |
| `--dry-run` | show what would run; the provider needn't be installed |
| `-q, --quiet` / `--silent` | less, or no, Claudine output |
| `--operation <OP>` (`--op`) | set `OPERATION` in the child's environment |
| `--repo` | expose only repo-scoped skills, commands, and agents (through a per-launch overlay) |
| `--sandbox` | provider-specific sandboxing |
| `--mcp`, `--use <ID,…>`, `--strict` | Claudine-managed MCP servers; see [config.md](config.md#mcp) |
| `--perf` | timing report on stderr |

- **Position never matters.** Every switch above is Claudine's anywhere
  before the first `--`, before or after the prompt
  (`claudine claude "fix it" -m opus` = `claudine claude -m opus "fix it"`).
  `-h`/`--help` anywhere before `--` shows help and exits 0.
- **Passthrough.** Unknown switches pass through to the provider. After
  `--`, nothing is read as a Claudine switch, and the `--` itself isn't
  forwarded.
- **No `--output`.** `-o`/`--output <anything>`, in any position before
  `--`, is refused with exit 2 and
  `the --output <target> CLI option has not been implemented yet in Claudine`
  on stderr, before anything runs. stdout is the provider's answer text;
  Claudine's own status goes to stderr.
- **Environment.**
  - Removed: secret-looking variables, and their names are listed on
    stderr.
  - Added: `AGENT`, `MODEL` (when known), `YOLO`, `INTERACTIVE`,
    `CLAUDINE_INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`,
    `CLAUDINE_PID`, `PACKAGE_AREA`, `PACKAGE`, and `OPERATION`.
  - `HOME` is never changed. An overlay sets the provider's own home
    variable, such as `CODEX_HOME`.
- **Output** for non-interactive runs:
  - stdout gets the agent's final answer, rendered as Markdown on a
    terminal.
  - stderr gets the header, environment changes, system prompt, thinking,
    tool calls (`🔧 →`/`🔧 ←`), and a summary.
- **Exit code.** The child's code is passed through, except that a failed
  result with native exit 0 returns `1`.

## `compose`, `inline-compose`, `sequence`

```bash
claudine compose        <file-ref> [key=value …] [switches] [-- provider args]
claudine inline-compose <file-ref> [key=value …] [switches]
claudine sequence       <file-ref> [key=value …] [switches]
```

`-h` on these three commands prints the top-level help, so use this table.
They take every wrapper switch above, plus:

| Switch | Effect |
|---|---|
| `--provider <P>` or `--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`, `--qwen` | choose the provider (beats frontmatter `agent`) |
| `--exclude <P>` | remove a provider from automatic selection (repeatable) |
| `--no-interactive` | force non-interactive |
| `--set '<JSON5 object>'` | override frontmatter (once; setters beat it) |
| `--dry-run` | print the composed body to stdout, and the resolved frontmatter plus a summary table to stderr. Launches nothing and fires no lifecycle event, but **`$(…)` expansions still run** |
| `--dry-run-shell` | with `--dry-run`, also run sequence `shell:` tasks for real |
| `--max-iterations <N>` | loop cap override |
| `--on-rate-limit pause\|abort\|continue` | rate-limit policy for loops (`abort` exits 75) |

- **Arguments.**
  - The first bare word that isn't a setter is the file. It must come
    before any provider switch.
  - Later bare words become `argv`, an array of strings.
  - Setters are `key=value`. The value is parsed as JSON5 (`count=3` is a
    number) and falls back to a string. A setter beats `--set`.
  - Everything after `--` goes to the agent untouched.
- **File references.** `./x.md`, a bare `x.md` (the launch folder, then the
  repository root), `&x.md` (repository root; quote it in the shell), `^x.md`,
  `~/x.md`, absolute paths, and `@x.md`.
  - `@` searches, in order: `{package}/prompts`, `{area}/prompts`,
    `{local}/prompts`, `{local}/.claudine/prompts`, `{local}/docs`, and the
    providers' `skills` folders, then `~/.claudine/prompts`, `~`, and
    `~/.claudine`.
  - So `claudine compose @commit.md` picks the closest `prompts/commit.md`.

## Steering and the dashboard

- `claudine steer "msg" [--session ID] [--list] [--json]` sends one message
  to a running session.
  - It needs the `rendezvous-daemon` running.
  - Today only a non-interactive Codex run that Claudine launched can
    receive messages.
  - Exit codes: `0` sent, `1` not sent, `2` invalid, `130` cancelled.
- When a wrapped agent starts repeating itself, Claudine nudges it
  automatically. Turn that off with `CLAUDINE_AUTO_STEER=off`.
- `claudine dashboard [--local]` shows live sessions across hosts.

## Inspection and admin

| Command | Use |
|---|---|
| `claudine hooks [provider] [--support\|--mapping\|--describe\|--variables]` | which hook events each provider has, and how they map to Claudine's events |
| `claudine actions` | configured hook actions, by event |
| `claudine sync [--dry-run] [--provider P] [--fix]` | re-register Claudine's hooks with installed agents (hooks only) |
| `claudine uninstall [--keep-config]` | remove Claudine's hooks |
| `claudine skills\|commands\|agents [FILTER…] [--fix] [-v]` | report shared resources; `--fix` links them across providers ([config.md](config.md#shared-resources)) |
| `claudine mcp …` | MCP catalog ([config.md](config.md#mcp)) |
| `claudine providers [--describe]` | provider capability matrix |
| `claudine logs [today\|week\|sessions\|tools\|errors\|trends\|session <id>…] [--json]` | query the logs in `~/.claudine/logs/` |
| `claudine context [--values\|--expressions\|--side-effects]` | the `ctx.*` variables (with current values), the expression functions, and the side-effect verbs available to documents |
| `claudine errors [--json]` | every diagnostic code, and what it means |
| `claudine doctor [--fix] [--json]` | check the whole setup in one run; see below |

### Checking the setup: `claudine doctor`

`claudine doctor` reports seven rows, always in this order: `config`,
`registration`, `darkmatter-permissions`, `repo-config`,
`stray-config-keys`, `tts`, `path`. Each row has a `severity` (`ok`,
`info`, `warn`, `error`) and remedies for what it found. It works with a
missing or broken configuration, never prompts, and without `--fix`
writes, creates, converts, or deletes none of your files: not Claudine's
or Darkmatter's configuration, not the published registration, and
nothing in your home or user profile (no default config, no `config.json`
conversion, no registration refresh). The one exception is on native
Windows: each PowerShell probe of the `path` row gets a private folder in
the system temporary directory for PowerShell's startup cache, deleted
when the probe ends. It makes no network request and plays no sound.
Reports never show configuration values or permission contents.

- Exit status: `0` when no row is an `error` (warnings are fine), `1` when
  any error remains (including a failed repair), `2` for a usage error.
- `--json` prints one array of `{id, severity, summary, detail, fixable}`
  objects; branch on `id`, `severity`, and `fixable`, not on the wording.
- `--fix` repairs only two rows: `registration` (republishes Claudine's
  format 3 schema bundle under `~/.claudine/registrations/darkmatter/` —
  upgrading an older format 1 or format 2 installation in place, writing
  `manifest.yaml` last, and restoring missing or modified members including
  any derived `expression-facts` companion — writes the `register` entry in
  an existing `~/.darkmatter.yaml` and drops any entry still naming the
  format 1 manifest, even after its file was deleted, then removes the
  format 1 `~/.claudine/darkmatter-host*` files) and
  `darkmatter-permissions` (moves an inline `permissions` block from
  `~/.darkmatter.yaml` into Claudine's file, as `claudine setup` does). The
  exit status reflects the state after the repairs, and a second `--fix`
  changes nothing. Everything else needs a manual fix; a missing
  configuration's remedy is `claudine setup`.
- `path` is an `error` when this process cannot find `claudine` on `PATH`.
  On native Windows it also asks each installed PowerShell, Git Bash/MSYS2,
  and Cygwin (without their startup files, 2 s limit); those results are
  at most `warn`.
- `tts` is an `error` only when hook speech is on (`tts: true` or an
  object) and no provider is available; it checks prerequisites, not a
  working voice.

```bash
claudine doctor --json | jq -r '.[] | select(.severity == "error") | .id'
```

## Environment variables you may set

| Variable | Effect |
|---|---|
| `CLAUDINE_YOLO` | same as `--yolo` |
| `CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT` (default 30m; `0s` off), `CLAUDINE_KILL_GRACE` (10s) | timeouts. Precedence: CLI flag > frontmatter > env > default |
| `CLAUDINE_MAX_ITERATIONS` | loop cap (below `--max-iterations`, above `loop.max`) |
| `CLAUDINE_AUTO_STEER` | `off` disables automatic steering |
| `PLAYA_DRY_RUN=1` | silences lifecycle speech and sounds |
| `MODEL`, `<PROVIDER>_MODEL` (e.g. `CODEX_MODEL`) | model defaults below `--model` |

## Files

- `~/.claudine/config.yaml`, and a repo `.claudine/config.yaml` (YAML,
  validated against `claudine/registration/claudine-config.yaml`; a legacy
  `config.json` is converted on the next load)
- `~/.claudine/logs/`, `~/.claudine/mcp/`, and `~/.claudine/overlays/`
- `~/.claudine/system-prompt.md` and a repo `system-prompt.md`
- `.claudine/non-interactive.md` and `.claudine/inline-compose.md`
- `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist`, at the
  prompt's repository root
