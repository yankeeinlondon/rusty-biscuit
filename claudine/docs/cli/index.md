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
  budget            Create and operate the shared budget ledger a sequence run enforces
  dashboard         Show the mesh NOW view: live sessions across rendezvous hosts
  completions       Generate shell completions
  context           Show Darkmatter runtime context, expression engine, and side effects
  errors            Show the diagnostic error-code contract (codes, dispositions, details)
```

## CLI Switch Overview

- all _switches_ start with a `--{name}` signature although in some cases you may find a _short form_ alias that takes the `-{alias}` format
- there is a **global** `--help`/`-h` switch that can be used anywhere in the CLI to get help; the help you get will be specific to where you in the API:
    - `claudine -h` - broad help overview listing commands, etc.
    - `claudine compose -h` - help on the **compose** command
- each command will also
