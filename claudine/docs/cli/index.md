# Claudine CLI

Claudine is a CLI that uses the frequently used _sub-_**command** model, the syntax structurally looks like:

```sh
claudine <command> [params] [switches]
```

## CLI Commands

You can run `claudine -h` to get a full list of the commands that looks like:

```sh
Shared Resources:
  skills            List available skills and their scopes
  commands          List available slash commands and their scopes
  agents            List available agent definitions and their scopes
  mcp               Manage MCP (Model Context Protocol) servers

Hook Events and Actions:
  hooks             Show registered hooks for all detected agents
  actions           Show configured actions and events

Wrapped Execution:
  claude            Wrap Claude Code with Claudine preflight/env handling
  codex             Wrap Codex CLI with Claudine preflight/env handling
  gemini            Wrap Gemini CLI with Claudine preflight/env handling
  goose             Wrap Goose with Claudine preflight/env handling
  kimi              Wrap Kimi Code with Claudine preflight/env handling
  opencode          Wrap OpenCode with Claudine preflight/env handling
  qwen              Wrap Qwen Code with Claudine preflight/env handling
  kilo              Wrap Kilo Code with Claudine preflight/env handling
  pi                Wrap Pi with Claudine preflight/env handling
  antigravity       Wrap Antigravity (the `agy` CLI) with Claudine preflight/env handling
  steer             Send a message to one running agent session

Composition:
  compose           Compose a Markdown document and send as prompt
  inline-compose    Inline composition: generate and replace body
  sequence          Run a serial sequence of composition steps from a single document

Administration:
  config            Manage Claudine configuration with a TUI
  sync              Re-sync hook registrations with detected agents
  uninstall         Remove Claudine hooks from all agents
  providers         Show provider capability matrix
  logs              Query and sync Claudine JSONL logs
  dashboard         Show the mesh NOW view: live sessions across rendezvous hosts
  completions       Generate shell completions
  context           Show Darkmatter runtime context, expression engine, and side effects
  errors            Show the diagnostic error-code contract (codes, dispositions, details)
```

## CLI Switch Overview

- all _switches_ start with a `--{name}` signature although in some cases you may find a _short form_ alias that takes the `-{alias}` format
- there is a **global** `--help`/`-h` switch that can be used anywhere in the CLI to get help; for most commands the help you get is specific to where you are in the API:
    - `claudine -h` - broad help overview listing commands, etc.
    - `claudine hooks -h`, `claudine mcp -h`, and the other administrative and shared-resource subcommands print their own help, and each provider wrapper (`claudine codex -h`, etc.) prints wrapper-specific help
    - `claudine compose -h`, `claudine inline-compose -h`, and `claudine sequence -h` print that command's own compact reference (see [Help for the composition commands](#help-for-the-composition-commands))

### Help for the composition commands

Where you put the help switch decides which screen you get:

| You type | You get |
| --- | --- |
| `claudine -h compose prompt.md` | the grouped overview, as for `claudine -h` (help comes before the command) |
| `claudine compose -h` | the `compose` reference |
| `claudine compose prompt.md name=Ken --help` | the `compose` reference; `prompt.md` is never opened |
| `claudine compose prompt.md -- --help` | an ordinary run: the document is composed and the provider launches with `--help` in its arguments |

The command reference shows the usage line (`Usage: claudine compose [OPTIONS] <ARG>...`), what the arguments after the command mean (one file reference, then optional `key=value` setters and arguments for the provider), every Claudine option including the ten provider shortcuts (`--claude` … `--antigravity`), and an example of forwarding to the provider. Provider-native options are not listed; the last row of the table shows how to pass the provider its own help switch.

```sh
# Help needs no file: neither of these reads, checks, or creates anything.
claudine compose -h
claudine sequence missing.md --help

# Disable color on either side of the command.
claudine --plain inline-compose --help
claudine inline-compose --help --plain
```

A help request after the command is answered before any other argument is checked. An unknown provider, a missing option value, malformed `--set` JSON, `--dry-run-shell` without `--dry-run`, or conflicting switches next to `--help` still print the reference and exit 0, with nothing on stderr. Nothing is composed, no configuration is created, no lifecycle action runs, and no provider launches. Only the first literal `--` changes this: everything after it belongs to the provider, so a `--help` there is forwarded instead of answered.

Shell completion is unaffected: completing a command line that contains `--help` lists candidates rather than printing help.
