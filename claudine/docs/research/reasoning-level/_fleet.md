---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/reasoning-level/{{state.file}}"
topic: Reasoning Level
# Research rotates over three agents by roster position. The order is chosen
# so that no agent researches its own provider.
agent: "{{ state.index % 3 == 1 ? 'opencode' : (state.index % 3 == 2 ? 'claude' : 'codex') }}"
model: "{{ state.index % 3 == 1 ? 'zai-coding-plan/glm-5.3' : (state.index % 3 == 2 ? 'sonnet' : 'gpt-6-luna') }}"
effort: "{{ state.index % 3 == 1 ? 'provider_default' : 'high' }}"
# Claudine cannot yet set reasoning effort from a prompt, so a run that must
# control it is made once per agent, each with that agent's own arguments:
#
#   claudine sequence _fleet.md -y pass=codex  -- -c model_reasoning_effort=high
#   claudine sequence _fleet.md -y pass=claude -- --effort high
#   claudine sequence _fleet.md -y pass=opencode
#
# With `pass` unset, one run covers every provider at each agent's default effort.
pass: all
# `only=<slug>` on the command line researches one provider, for a pilot or a repair.
only: all
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "state.skip_research == true"
          action:
              - stderr: "**{{state.name}}** is marked `skip_research`; skipping {{topic}}"
              - skip
        - when: "only != 'all' && only != state.slug"
          action:
              - skip
        - when: "pass != 'all' && pass != agent"
          action:
              - stderr: "**{{state.name}}** is assigned to {{agent}}, not this pass; skipping"
              - skip
        - when: "file_exists(file) && frontmatter(file, 'schema_revision') == 1 && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "**{{state.name}}** has current {{topic}} research; skipping"
              - skip
        - action:
              - info: "Researching **{{topic}}** for **{{state.name}}** with {{agent}} / {{model}} (effort: {{effort}})"
start:
    stack:
        - when: "!has_binary(state.binary)"
          action:
              - warn: "**{{state.binary}}** is not installed on this host, so the research cannot inspect it locally"
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - warn: "The agent finished, but {{ link(file) }} was not updated today"
              - error: "the research document was not updated"
        - when: "frontmatter(file, 'agent') != agent || frontmatter(file, 'model') != model"
          action:
              - warn: "{{ link(file) }} records a different agent or model than the one assigned ({{agent}} / {{model}})"
              - error: "the research document records the wrong agent or model"
        - action:
              - set:
                    contract: "$(md schema validate '{{file}}' --no-trigger-schemas --format json)::result"
        - when: "!contract.ok"
          action:
              - warn: "{{topic}} research for **{{state.name}}** does not satisfy its contract"
              - stderr: "{{{ contract.stdout }}}"
              - error: "the research document does not satisfy its contract"
        - action:
              - success: "{{topic}} research for **{{state.name}}** satisfies its contract: {{ link(file) }}"
              - message: "✅ {{topic}} research for **{{state.name}}** completed and validated"
failure:
    stack:
        - action:
              - warn: "{{topic}} research for **{{state.name}}** failed: {{{ err.message }}}"
              - message: "💥 {{topic}} research for **{{state.name}}** failed: {{{ err.message }}}"
finalize:
    stack:
        # One more attempt. The retried run reads `contract`, so the agent
        # sees exactly which properties were rejected and why.
        - when: "err"
          action: { retry: 1 }
---
# Reasoning Level Research on {{state.name}}

You are the assigned researcher, already running inside the fleet. Do the
research yourself. Do not launch another agent or another research run.

## Skills

Use the 'claudine' skill.

## Scope

This topic covers how **{{state.name}}** lets a caller choose how much
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

::block when="contract && !contract.ok"
## Your previous attempt was rejected

The document you wrote did not satisfy the contract. Fix every problem below.
Each one names the property, what is wrong, and what the property is for.

```json
{{contract.stdout}}
```

::end-block
## The Contract

Read `./_schema.yaml`, `./_types.yaml`, and `../_types.yaml` before writing.
They define every property, its allowed values, and what to record in it. The
fleet validates your document against them when you finish and rejects a
document that does not conform.

- Set `$schema: ./_schema.yaml` and `schema_revision: 1`.
- Set `provider: {{state.slug}}`, `agent: {{agent}}`, `model: {{model}}`, and
  `reasoning_effort: {{effort}}`.
- Set `last_updated: {{ctx.today}}`. Keep `created` unchanged when the
  document already exists.
- When a fact cannot be established, record `unknown` and add an entry to
  `gaps` that names the check that would settle it. Never guess, and never
  leave a required property out.
- Every finding cites `evidence_ids`. Inference alone does not establish a
  level, a control, or a default.

## What to Establish

1. **Levels.** Which level tokens does {{state.name}} accept? List them
   weakest first and map each to the closest value of the contract's
   provider-neutral scale, using the provider's own ordering.
2. **Default.** Which level applies when the caller chooses nothing, and does
   that depend on the model or the account?
3. **Controls.** Every way to choose a level: launch flags, configuration
   override flags, configuration keys, environment variables, a suffix on the
   model name, commands typed inside a session, and fields of a request. For a
   control usable at launch, give the exact arguments.
4. **Precedence.** When several controls are set, which wins? Give evidence
   for the order; do not assume flags beat variables beat files.
5. **Models.** Which models accept which levels? Record each model whose
   levels differ from the full list.
6. **An invalid level.** Request a level the provider does not accept and
   record what happens: a refusal, a failed request, a silent fallback.
7. **Reporting.** Where does the provider state the level a run actually
   used? This is how Claudine will confirm a request took effect.
8. **Reasoning output.** Does a non-interactive caller receive the reasoning
   text, a summary, or nothing?

## Method

::block when="update"
- Read the existing research in `{{file}}` so you can describe what changed.
  Treat it as out of date; never copy a finding from it without checking.
::end-block
- Run `{{state.binary}} --version` and record the exact version in
  `versions_examined`. When {{state.name}} is not installed on this host, say
  so in `gaps` and rely on documentation and source.
- Inspect the provider's help output, its configuration under
  `{{state.user_dir || 'the provider configuration directory'}}`, and its
  documentation at {{state.site}}. Prefer what you observe over what
  documentation claims.
- You may run {{state.name}} in a disposable non-interactive session to test a
  level. Do not change the user's configuration files.
- A check that finds nothing is a finding. Say what you checked.

## Document Body

After the frontmatter, write these sections for a reader who has not used
{{state.name}}:

- `## Levels` — the levels and what each does
- `## Choosing a Level` — each control, with one working example
- `## Models` — which models accept which levels
- `## Confirming the Level` — how to verify what a run used
- `## Sources` — links to everything you relied on
- `## Changelog` — what changed since the previous version
