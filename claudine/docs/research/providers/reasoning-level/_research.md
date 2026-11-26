---
file: "{{ctx.repo_root}}/claudine/docs/research/reasoning-level/{{seq.file}}"
topic: Reasoning Level
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
# each run naming its agent, model, and effort. `just research <topic>` does
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
    # `contract_checked`; a document the fleet rejected never carries it.
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
    - set:
          relations: "$(python3 '{{ctx.repo_root}}/claudine/docs/research/reasoning-level/_relations.py' '{{file}}')::result"
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
# Reasoning Level Research on {{seq.name}}

You are the assigned researcher, already running inside the fleet. Do the
research yourself. Do not launch another agent or another research run.

## Skills

Use the 'claudine' skill.

## Scope

This topic covers how **{{seq.name}}** lets a caller choose how much
reasoning a model applies: which levels exist, how each is requested, which
models accept which levels, and how to confirm afterwards which level a run
used. Providers call this effort, thinking, reasoning, or variant.

Claudine will use the result to offer one effort setting that works on every
provider. It can only do that from exact values, so copy every flag, key,
variable, and level token exactly as you observe it.

Model selection belongs to the `agent-models` topic. Name a model here only to
record which levels it accepts.

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
either. Run both yourself before you finish:

```sh
md schema validate '{{file}}' --disable schema-triggers
python3 '{{ctx.repo_root}}/claudine/docs/research/reasoning-level/_relations.py' '{{file}}'
```

- Set `$schema: ./_schema.yaml` and `schema_revision: 2`.
- Set `provider: {{seq.slug}}`, `agent: {{agent}}`, `model: {{model}}`, and
  `reasoning_effort: {{effort}}`.
- Set `last_updated: {{ctx.today}}`. Keep `created` unchanged when the
  document already exists.
- Do not write `contract_checked`. The fleet writes it after your document
  passes validation.
- When a fact cannot be established, record `unknown` and add an entry to
  `gaps` that names the check that would settle it. Never guess, and never
  leave a required property out. A gap names its `area`, and its `entry` when
  it concerns one level, control, or model.
- Every finding cites `evidence_ids`. Inference alone does not establish a
  level, a control, or a default.

## What to Establish

1. **Levels.** Which level tokens does {{seq.name}} accept? List the levels
   of its scale weakest first and map each to the closest value of the
   contract's provider-neutral scale, using the provider's own ordering. A
   mode that is not a point on the scale, such as one that also changes how
   the agent works, goes after them with `outside_scale`.
2. **Default.** Which level applies when the caller chooses nothing, and does
   that depend on the model or the account?
3. **Controls.** Every way to choose a level: launch flags, configuration
   override flags, configuration keys, environment variables, a suffix on the
   model name, commands typed inside a session, and fields of a request.
   `arguments` holds command-line arguments only, one per entry. For an
   environment variable or a configuration key, `name` is enough and
   `arguments` stays empty.
4. **Precedence.** When several controls are set, which wins? Give evidence
   for the order; do not assume flags beat variables beat files.
5. **Models.** Which models accept which levels? Record only models whose
   levels differ from the full list. Group models that share the same levels
   under one pattern, such as `gpt-6-*`. Do not copy a model catalog; 40
   entries is the limit.
6. **An invalid level.** Request a level the provider does not accept and
   record what happens: a refusal, a failed request, a silent fallback.
7. **Reporting.** Where does the provider state the level a run actually
   used? This is how Claudine will confirm a request took effect. A file
   pattern or an event name goes in `locator`; a command goes in `command`,
   one argument per entry.
8. **Reasoning output.** Does a non-interactive caller receive the reasoning
   text, a summary, or nothing?

## Method

::block when="update"
- Read the existing research in `{{file}}` so you can describe what changed.
  Treat it as out of date; never copy a finding from it without checking.
::end-block
- Run `{{seq.binary}} --version` and record the exact version in
  `versions_examined`. When {{seq.name}} is not installed on this host, say
  so in `gaps` and rely on documentation and source.
- Inspect the provider's help output, its configuration under
  `{{seq.user_dir || 'the provider configuration directory'}}`, and its
  documentation at {{seq.site}}. Prefer what you observe over what
  documentation claims.
- You may run {{seq.name}} in a disposable non-interactive session to test a
  level. Do not change the user's configuration files.
- A check that finds nothing is a finding. Say what you checked.

## Document Body

After the frontmatter, write these sections for a reader who has not used
{{seq.name}}:

- `## Levels` — the levels and what each does
- `## Choosing a Level` — each control, with one working example
- `## Models` — which models accept which levels
- `## Confirming the Level` — how to verify what a run used
- `## Sources` — links to everything you relied on
- `## Changelog` — what changed since the previous version
