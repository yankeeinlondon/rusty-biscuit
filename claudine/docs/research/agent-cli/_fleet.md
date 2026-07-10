---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-cli/{{state.file}}"
yolo: true
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **CLI**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **CLI** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action: 
              - info: "The **Agent CLI** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Agent CLI** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent CLI research on **{{state.name}}** failed to complete!"
    warn: "The Agent CLI research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Agent CLI Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research the public CLI surface for **{{state.desc}}**. This topic feeds Claudine's
provider metadata and wrapper implementation: binary names, installation paths,
subcommands, flags, config discovery, machine-readable introspection, runtime env vars,
and wrapper-impacting caveats.

**Boundary:** the sibling `system-prompt` topic owns system-prompt delivery flags in
depth (replace-vs-append semantics, file-vs-inline forms, mode interactions). This
topic's switch inventory records that those flags *exist* — flag name, value shape, one
example — and defers their semantics to that topic; do not duplicate its research here.

## Document Structure

The research deliverable is a prose document a maintainer can learn the provider's CLI
surface from. Write the body of `{{file}}` using these sections; frontmatter is
distilled from this body afterward, never invented separately.

- `## Overview` Section
    - What the CLI is, who ships it, and the primary command a user types
    - The current upstream version you verified — and how you verified it (release
      notes, package registry, local `--version`) — or an explicit statement that it
      is not discoverable
    - The primary homepage, repository, general docs, and CLI-reference URLs; prefer
      official sources
- `## Installation and Binaries` Section
    - Binary names, aliases, and shims per OS — Windows installs often expose
      `.exe`/`.cmd` shims that differ from the macOS/Linux command name
    - How the CLI is installed on each OS, with the exact commands the docs provide
      (brew, npm, winget, cargo, standalone installer, …)
    - Only present a single cross-platform answer when the command name and install
      path really are identical everywhere
- `## Subcommands` Section
    - Every top-level subcommand or mode the binary exposes, each with a one-line
      description
    - Say which commands are intended to run a prompt/task without a TTY (the
      automation entry points) versus which need a TTY, a browser, or user
      interaction (logins, pickers, OAuth flows)
- `## CLI Switch Inventory` Section
    - The full switch inventory: global flags and subcommand-specific flags, with
      defaults and a concrete example invocation for the wrapper-relevant ones
    - Be explicit about which switches are global versus scoped to a subcommand, and
      which are boolean versus value-taking
    - `--help` output sometimes omits documented flags; when help output and official
      docs disagree, say which one you trusted and why
    - Record the existence of system-prompt delivery flags here, but link to the
      `system-prompt` topic instead of re-documenting their semantics (see Scope)
- `## Configuration Discovery` Section
    - Which config files the CLI discovers or exposes, at which scopes (user / repo /
      system / env), in what format, with per-OS paths when they differ
    - Note config side effects a wrapper should know about: files the CLI writes on
      first run, trust prompts, state directories
- `## Environment Variables` Section
    - Only general CLI/runtime variables belong here: do not duplicate model-endpoint
      variables from `model-config`, permission variables from `agent-permissions`,
      MCP variables from `mcp`, logging variables from `agent-logging`, or streaming
      variables from `streaming` unless they also affect general CLI behavior
    - For each variable, state its concrete effect, not just that it exists
- `## Machine Introspection` Section
    - Commands Claudine could run to discover provider state for wrappers, reports,
      or codegen: model catalogs, config dumps or schemas, doctor diagnostics,
      effective env/config reports, plugin/extension lists, MCP server lists, tool
      lists, capability reports
    - Do not pad this with generic `--help`/`--version` entries unless they expose
      machine-usable data
    - For each command, say whether its output is machine-readable, what format it
      uses, and whether it is useful for code generation
- `## Wrapper Notes` Section
    - Concrete caveats for Claudine wrappers, not general commentary: noisy stderr
      during successful runs, TTY requirements, shell-quoting hazards, config side
      effects, broken flags, platform differences, auth requirements, and non-zero
      exits for expected states
- `## Changelog` Section (update runs only)
    - Summarize what changed since the prior research
- `## Sources`
    - add all useful resources you used as Markdown links — official docs, help
      output, release notes, and the local inspection commands you ran

Do not add thinking or preparatory statements to the document body. Those can go to
stdout during the run, but the saved Markdown body must contain only the research.

**IMPORTANT:** DO NOT MAKE THINGS UP. It is far better to admit you don't know
something than to make up something just to "complete" the exercise!

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLIs change is rapid and therefore you
    > should assume that the prior research is out of date. You are reading this
    > primarily to be able to effectively report the changes into the `## Changelog`
    > section of the document. Critically, you should never substitute information in
    > the old research for doing your own (up-to-date) research.

::end-block
- Perform research on the topic

    > **Evidence requirement:** you have read access to `{{state.user_dir}}` on this
    > host. Inspect the actual installed binary, its help output, and the config files
    > there, and prefer what you observe over what documentation claims. Negative
    > probes are evidence too — "the installed version rejects this flag" is a
    > finding. Unanswered is not the same as omitted: record `unknown` (or an empty
    > array) with a body note rather than dropping a field.

::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save the research to `{{file}}`, following the Document Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative field contract, and `md schema validate` will enforce it against
    > everything you write.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `latest_version` - the version you verified in `## Overview`, or `unknown`. Do
      not use the old `latest-version` key; the schema property is `latest_version`
    - `homepage`, `repo`, `docs`, `cli_docs` - the URLs cited in `## Overview`
    - `binaries` and `install_methods` - distilled from `## Installation and Binaries`;
      one record per OS for every os-bearing record in this document — never `os: all`
      (Windows binaries and install commands always differ)
    - `subcommands` - from `## Subcommands`, with `non_interactive` reflecting your
      TTY analysis
    - `cli_switches` - the inventory from `## CLI Switch Inventory`
    - `config_paths` - from `## Configuration Discovery`; one record per OS — file
      paths must be recorded separately for macOS, Linux, and Windows (never one
      record for all OSes; Windows paths always differ)
    - `env_vars` - from `## Environment Variables`
    - `machine_introspection` - from `## Machine Introspection`
    - `wrapper_notes` - from `## Wrapper Notes`
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** based on the changes discovered in your research.
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document, following the Document Structure
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
