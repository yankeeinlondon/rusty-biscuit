---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-cli/{{state.file}}"
agent: opencode
model: kimi-for-coding/k2p7
yolo: true
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> CLI is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action: 
              - info: "The Agent CLI research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the Agent CLI research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent CLI research on **{{state.name}}** failed to complete!"
    warn: "The Agent CLI research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---

## Skills

Use the 'claudine' skill.

## Scope

Research the public CLI surface for **{{state.desc}}**. This topic feeds Claudine's
provider metadata and wrapper implementation: binary names, installation paths,
subcommands, flags, config discovery, machine-readable introspection, runtime env vars,
and wrapper-impacting caveats.

Write the result to `{{file}}` and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `latest_version`
- `homepage`, `repo`, `docs`, `cli_docs`
- `binaries`
- `install_methods`
- `subcommands`
- `cli_switches`
- `config_files`
- `env_vars`
- `machine_introspection`
- `wrapper_notes`
- `changes`
- `requires_claudine_update`
- `reason`

Use `unknown`, empty arrays, or a clear body note when current documentation and CLI
inspection do not prove a value. Do not use the old `latest-version` key; the schema
property is `latest_version`.

## Frontmatter Field Guide

Use this section as the authoritative meaning of each schema property.

### Identity and Links

- `created`: Date this provider file was first created. Set only on first creation.
  Example: `created: 2026-07-02`
- `last_updated`: Date this research was verified. Always set to `{{ctx.today}}`.
- `agent`: Research runner. Set to `{{env.AGENT}}`.
- `model`: Research model. Set to `{{env.MODEL || 'default'}}`.
- `latest_version`: Current upstream CLI version, or `unknown` if not discoverable.
  Example: `latest_version: "1.2.3"`
- `homepage`, `repo`, `docs`, `cli_docs`: Primary URLs. Prefer official docs and
  source repositories.

### Binaries

`binaries` records executable names by OS. Use `os: all` only when the command name is
the same everywhere.

Example:

```yaml
binaries:
  - os: all
    binary: codex
    alt_binaries: []
    notes: "Primary CLI executable installed by npm and Homebrew."
  - os: windows
    binary: codex.exe
    alt_binaries: ["codex.cmd"]
    notes: "Windows npm installs may expose a .cmd shim."
```

### Install Methods

`install_methods` records how a user installs the CLI on each OS. Capture exact commands
when docs provide them.

Example:

```yaml
install_methods:
  - os: macos
    method: brew
    command: "brew install codex"
    notes: "Preferred macOS package-manager install."
  - os: all
    method: npm
    command: "npm install -g @openai/codex"
    notes: "Cross-platform Node.js install."
```

### Subcommands

`subcommands` is one record per top-level command or mode the binary exposes. Mark
`non_interactive: true` when the command is intended to run a prompt/task without a TTY
conversation.

Example:

```yaml
subcommands:
  - name: run
    description: "Runs a prompt non-interactively and exits."
    non_interactive: true
    notes: "Primary automation entry point."
  - name: auth
    description: "Manages authentication."
    non_interactive: false
    notes: "May launch a browser or prompt interactively."
```

### CLI Switches

`cli_switches` is the full switch inventory. Include global flags and subcommand-specific
flags. Put the command or topic in `scope`; keep it free-form, but be consistent.

Examples:

```yaml
cli_switches:
  - flag: --model
    value: "<MODEL>"
    scope: ["global", "model_selection"]
    default: "provider default"
    description: "Selects the model for the session."
    example: "codex --model gpt-5"
    notes: "May also be configurable via config file."
  - flag: --json
    value: ""
    scope: ["run", "output"]
    default: "false"
    description: "Emits machine-readable JSON output."
    example: "codex run --json \"summarize this repo\""
    notes: "Use an empty value for boolean switches."
```

### Config Files

`config_files` records config discovery exposed by the CLI itself. Use per-OS records
when paths differ. `scope` describes whose config it is, not what feature the config
controls.

Example:

```yaml
config_files:
  - os: macos
    scope: user
    path: "~/.codex/config.toml"
    format: toml
    notes: "Primary user config."
  - os: all
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Repo-local config if supported."
```

### Environment Variables

`env_vars` is only for general CLI/runtime variables not owned by a narrower topic. Do
not duplicate model endpoint variables from `model-config`, permission variables from
`agent-permissions`, MCP variables from `mcp`, logging variables from `agent-logging`,
or streaming variables from `streaming` unless they also affect general CLI behavior.

Example:

```yaml
env_vars:
  - name: NO_COLOR
    effect: "Disables ANSI color output."
  - name: CODEX_HOME
    effect: "Overrides the CLI home/config directory."
```

### Machine Introspection

`machine_introspection` records commands Claudine could run to discover provider state
for wrappers, reports, or codegen. A command is useful when output can be parsed or
contains stable state. Avoid generic help/version entries unless they provide structured
or otherwise useful information.

Example:

```yaml
machine_introspection:
  - command: "codex models --json"
    purpose: models
    machine_readable: true
    output_format: json
    useful_for_codegen: true
    notes: "Enumerates accepted model ids."
  - command: "codex doctor --json"
    purpose: doctor
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Useful for diagnostics, not static provider metadata."
```

### Wrapper Notes and Change Flags

- `wrapper_notes`: Concrete caveats for Claudine wrappers, not general commentary.
  Examples: `"Writes progress to stderr during successful runs."`,
  `"Requires a TTY for login."`, `"JSON output is mixed with human text unless --quiet is set."`
- `changes`: Update-mode changelog entries. Fresh first-run docs should use `[]`.
- `requires_claudine_update`: Set `true` only when the research implies a Claudine code
  or generated metadata change, not merely because documentation changed.
- `reason`: Required when `requires_claudine_update` is `true`; otherwise use an empty
  string or omit if the schema allows.

## Research Questions

- What binary names, aliases, or shims exist on macOS, Linux, and Windows?
- How is the CLI installed on each OS?
- What top-level subcommands or modes does the CLI expose?
- What is the full CLI switch inventory, including defaults and examples?
- Which switches are global versus subcommand-specific?
- Which config files does the CLI discover or expose, and in what format?
- Which general CLI/runtime environment variables affect behavior but are not better
  owned by another topic such as model-config, permissions, MCP, logging, or streaming?
- Which commands can Claudine run to discover machine-usable state?
- What caveats matter for wrappers: noisy stderr, TTY requirements, quoting, config
  side effects, broken flags, platform differences, auth requirements, or non-zero
  exits for expected states?

## Machine Introspection Guidance

Do not fill `machine_introspection` with generic `--help` and `--version` entries unless
they expose machine-usable data. Prefer commands that reveal provider state useful to
wrappers or codegen:

- model catalogs
- config dumps or config schemas
- doctor diagnostics
- effective env/config reports
- plugin/extension lists
- MCP server lists
- tool lists
- capability or feature reports

For each command, record whether it is machine-readable, its output format, and whether
it is useful for code generation.

## Body Structure

- `## Overview`
- `## Installation and Binaries`
- `## Subcommands`
- `## CLI Switch Inventory`
- `## Configuration Discovery`
- `## Environment Variables`
- `## Machine Introspection`
- `## Wrapper Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation, `--help`/subcommand help, release notes, and local
inspection where available. Cite sources as Markdown links.

Do not add thinking or preparatory statements to the document body. Those can go to
stdout during the run, but the saved Markdown body must contain only the research.

**IMPORTANT:** DO NOT MAKE THINGS UP. It is far better to admit you don't know something than to make up something just to "complete" the exercise!
