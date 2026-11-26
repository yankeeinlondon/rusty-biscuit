---
file: "{{ctx.repo_root}}/claudine/docs/research/agent-cli/{{seq.file}}"
topic: Agent CLI
# Research rotates over three researchers by roster position. The order is
# chosen so that no agent researches its own provider. This is the only place
# the rotation is written; `agent`, `model`, and `effort` follow from it.
assigned: "{{ seq.index % 3 == 1 ? 'opencode' : (seq.index % 3 == 2 ? 'claude' : 'codex') }}"
# `covering=<agent>` on the command line names a researcher that has reached
# its usage limit. The agent this run launches then researches that
# researcher's providers as well as its own, except a provider that is itself.
covering: none
agent: "{{ covering != 'none' && assigned == covering ? env.AGENT : assigned }}"
model: "{{ agent == 'opencode' ? 'zai-coding-plan/glm-5.3' : (agent == 'claude' ? 'sonnet' : 'gpt-6.1-sol') }}"
effort: "{{ agent == 'opencode' ? 'provider_default' : (agent == 'claude' ? 'high' : 'medium') }}"
# A sequence plans one agent before any step exists, and Claudine cannot yet
# set reasoning effort from a prompt. So the fleet runs once per researcher,
# each run naming its agent, model, and effort. `just research agent-cli` does
# this, and covers for a researcher that reaches its usage limit.
# `only=<slug>` on the command line researches one provider, for a pilot or a repair.
only: all
# Read only the documents this run writes. Another run may be in the middle
# of writing one of the others.
update: "{{ env.AGENT == agent && file_exists(file) && !markdown_body_empty(file) }}"
initialize:
    - when: "seq.skip_research == true"
      then:
          - stderr: "**{{seq.name}}** is marked `skip_research`; skipping {{topic}}"
          - skip
    - when: "only != 'all' && only != seq.slug"
      then:
          - skip
    - when: "env.AGENT != agent"
      then:
          - stderr: "**{{seq.name}}** is assigned to {{agent}}, and this run launches {{env.AGENT}}; skipping"
          - skip
    - when: "agent == seq.slug"
      then:
          - stderr: "**{{seq.name}}** would be researched by its own agent; skipping"
          - skip
    - when: "env.MODEL != model"
      then:
          - warn: "**{{seq.name}}** is assigned {{model}}, and this run launches {{env.MODEL}}"
          - error: "this run launches a model outside the rotation"
    # Current means the fleet confirmed the document against this revision
    # of the contract within 14 days. The `success` event writes
    # `contract_checked`; a document the fleet rejected never carries it,
    # and a document written for an older revision never skips.
    - when: "file_exists(file) && frontmatter(file, 'schema_revision') == 2 && frontmatter(file, 'contract_checked') && !date_delta(frontmatter(file, 'contract_checked'), ctx.today, '14d')"
      then:
          - stderr: "**{{seq.name}}** has {{topic}} research confirmed on {{ frontmatter(file, 'contract_checked') }}; skipping"
          - skip
    - info: "Researching **{{topic}}** for **{{seq.name}}** with {{agent}} / {{model}} (effort: {{effort}}){{ agent != assigned ? ', covering for ' + assigned : '' }}"
start:
    - when: "!has_binary(seq.binary)"
      then:
          - warn: "**{{seq.binary}}** is not installed on this host, so the research cannot inspect it locally"
success:
    - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
      then:
          - warn: "The agent finished, but {{ link(file) }} was not updated today"
          - error: "the research document was not updated"
    - when: "frontmatter(file, 'agent') != agent || frontmatter(file, 'model') != model"
      then:
          - warn: "{{ link(file) }} records a different agent or model than the one assigned ({{agent}} / {{model}})"
          - error: "the research document records the wrong agent or model"
    - set:
          contract: "$(md schema validate '{{file}}' --disable schema-triggers --format json)::result"
    - when: "!contract.ok"
      then:
          - warn: "{{topic}} research for **{{seq.name}}** does not satisfy its contract"
          - stderr: "{{{ contract.stdout }}}"
          - error: "the research document does not satisfy its contract"
    # The relations script also runs `claudine-gen validate`, which owns
    # the relations between switch records.
    - set:
          relations: "$(python3 '{{ctx.repo_root}}/claudine/docs/research/agent-cli/_relations.py' '{{file}}')::result"
    - when: "!relations.ok"
      then:
          - warn: "{{topic}} research for **{{seq.name}}** breaks a relation its contract requires"
          - stderr: "{{{ relations.stdout }}}"
          - error: "the research document breaks a relation its contract requires"
    - set_frontmatter: ["{{file}}", "contract_checked", "{{ctx.today}}"]
    - success: "{{topic}} research for **{{seq.name}}** satisfies its contract: {{ link(file) }}"
    - message: "✅ {{topic}} research for **{{seq.name}}** completed and validated"
failure:
    - warn: "{{topic}} research for **{{seq.name}}** failed: {{ err.msg }}"
    - message: "💥 {{topic}} research for **{{seq.name}}** failed: {{ err.msg }}"
finalize:
    # A usage limit is not cured by trying again. The provider stays
    # unresearched until the limit resets.
    - when: "err && err.category == 'cap'"
      then:
          - warn: "{{env.AGENT}} has reached a usage limit, so **{{seq.name}}** was not researched"
    # Anything else gets one more attempt. The retried run reads what the
    # gates rejected, so the researcher sees what to correct.
    - when: "err && err.category != 'cap'"
      then:
          - retry: 1
---
# Agent CLI Research on {{seq.name}}

You are the assigned researcher, already running inside the fleet. Do the
research yourself. Do not launch another agent or another research run.

## Skills

Use the 'claudine' skill.

## Scope

Research the public command-line surface of **{{seq.desc}}**: binary names,
installation, subcommands, switches, configuration discovery, runtime
environment variables, machine-readable introspection, and caveats for a
program that wraps the CLI.

The switch inventory matters most. Claudine reads a command line that mixes
its own inputs with switches meant for {{seq.name}}, and decides which
argument belongs to whom from what you record: whether a switch takes a value,
how many, and in which written forms. A wrong type makes Claudine take a
user's argument away from them, so copy every spelling exactly as you observe
it, and record `unknown` rather than guess.

**Boundary:** the `system-prompt` topic owns system-prompt delivery flags in
depth. Record that those flags exist, with their spellings and value type, and
leave their semantics to that topic. Model-endpoint variables belong to
`model-config`, permission variables to `agent-permissions`, MCP variables to
`mcp`, and logging variables to `agent-logging`.

Other providers' documents in this directory are research outputs, not
sources. Do not open or cite them.

::block when="(contract && !contract.ok) || (relations && !relations.ok)"
## Your previous attempt was rejected

The document you wrote was rejected. Fix every problem below and nothing
else. Each one names the property and what is wrong with it.

::end-block
::block when="contract && !contract.ok"
```json
{{contract.stdout}}
```

::end-block
::block when="relations && !relations.ok"
```text
{{relations.stdout}}
```

::end-block
## The Contract

Read `./_schema.yaml`, `./_types.yaml`, and `../_types.yaml` before writing.
They define every property, its allowed values, and what to record in it.

The fleet runs two checks when you finish and rejects a document that fails
either. The second also runs the provider-catalog generator over your switch
records. Run both yourself before you finish:

```sh
md schema validate '{{file}}' --disable schema-triggers
python3 '{{ctx.repo_root}}/claudine/docs/research/agent-cli/_relations.py' '{{file}}'
```

- Set `$schema: ./_schema.yaml` and `schema_revision: 2`.
- Set `provider: {{seq.slug}}`, `agent: {{agent}}`, `model: {{model}}`, and
  `reasoning_effort: {{effort}}`.
- Set `last_updated: {{ctx.today}}`. Keep `created` unchanged when the
  document already exists; otherwise set it to {{ctx.today}}.
- Do not write `contract_checked`. The fleet writes it after your document
  passes validation.
- Every switch record cites `evidence_ids`. Inference alone does not establish
  a value type. Every version an evidence entry describes is listed in
  `versions_examined`.
- When a switch's value type or a variadic minimum cannot be established,
  record `unknown` and say in the record's `gap` which check would settle it.
  Never guess, and never leave a required property out.

## What to Establish

1. **Version.** Run `{{seq.binary}} --version` and record the exact version
   in `versions_examined`. Record the newest released version you find
   upstream in `latest_version`. When {{seq.name}} is not installed on this
   host, say so in `wrapper_notes` and rely on documentation and source.
2. **Command paths.** List every native command path below the executable in
   `subcommands`, as words separated by one space (`exec resume`). The root
   entrypoint, the bare executable, is not a subcommand. Mark each path
   `non_interactive` only when it runs to completion with no terminal,
   browser, or person answering a prompt.
3. **Switches.** Record every switch accepted at the root entrypoint and at
   every path you marked `non_interactive`, including resume paths. Switches of
   other paths, such as login or plugin management, may be left out. For each
   switch:
   - `flag` is the canonical spelling and `aliases` every other spelling of
     the same switch, such as `-c` for `--config`.
   - `value_type`: `none` takes no value; `string` and `number` take one;
     `variadic` takes several in one occurrence (`--image a.png b.png`). A
     switch that may be repeated, each time with one value, is `string`, not
     `variadic`. `value_optional` says whether the one value may be left out.
     `variadic_min` is the fewest values a variadic switch requires.
   - `attachment` lists every form the parser accepts: `space` (`--config
     x=y`), `equals` (`--config=x=y`), and `short_attached` (`-cx=y`, only
     for a single-dash, one-character spelling).
   - `invocation_scope`: `applies_to: global` when the switch is accepted at
     every command path; otherwise one `applies_to: command` entry per exact
     path, with `command: []` for the root entrypoint.
4. **Configuration, environment, introspection, and caveats.** Record the
   configuration files the CLI reads, one record per operating system; the
   general runtime variables and what each changes; the commands that report
   machine-readable provider state; and caveats for a wrapper.

## Method

::block when="update"
- Read the existing research in `{{file}}` so you can describe what changed.
  Treat it as out of date; never copy a finding from it without checking.
::end-block
- Inspect the installed binary's help output at every command path you
  inventory, its configuration under
  `{{seq.user_dir || 'the provider configuration directory'}}`, its
  documentation at {{seq.site}}, and its source where it is published.
  Prefer what you observe over what documentation claims.
- Establish value consumption and attachment forms from the argument parser:
  name the parsing library and the declaration in source, or run a disposable
  test. A test proves a form only when its result shows the parser read the
  value, for example an invalid value rejected with an error naming the
  switch; a run that exits before parsing, such as `--help`, proves nothing.
- You may run {{seq.name}} non-interactively to test parsing. Do not start a
  model session that costs money, and do not change the user's configuration
  files.
- A check that finds nothing is a finding. Say what you checked.

## Document Body

After the frontmatter, write these sections for a reader who has not used
{{seq.name}}. Frontmatter is distilled from the body, never invented
separately.

- `## Overview` — what the CLI is, who ships it, the version you verified and
  how, and its main links
- `## Installation and Binaries` — command names and installs per operating
  system
- `## Subcommands` — each command path, and which run without a terminal
- `## CLI Switch Inventory` — the switches by command path, how each takes its
  value, and which source or test established it; say which paths you
  inventoried
- `## Configuration Discovery` — the files the CLI reads and writes
- `## Environment Variables` — each variable and its effect
- `## Machine Introspection` — commands with machine-readable output
- `## Wrapper Notes` — caveats for a program that wraps the CLI
- `## Sources` — links to everything you relied on
- `## Changelog` — what changed since the previous version

Do not add thinking or preparatory statements to the body.

## Output

::file @prompts/make-it-markdown.md
