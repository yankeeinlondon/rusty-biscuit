---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-models/{{state.file}}"
agent: opencode
model: kimi-for-coding/k2p7
# the frontmatter contract for target documents lives in the schema sidecar
# (./_schema.yaml) so the contract is single-sourced and machine-validated
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
# make interrupted fleet runs resumable: skip providers already researched today
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> is already up to date ({{ctx.today}}) — skipping."
              - skip
# a provider exiting 0 is not proof the research was written — verify the
# agent actually stamped today's date before accepting success
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
---

## Skills

Use the 'claudine' skill.

## Document Structure

Your job is to do detailed research into the **model** support in the **{{state.desc}}** solution. You are expected to answer the following questions:

- `## Model's Available` Section
    - what models are available by default when you install **{{state.name}}**?
    - how can you add in bespoke models (local models or otherwise)?

- `## Model Configuration Details` Section

    - Does **{{state.name}}** provide a formal schema for the configuration of its models? An informal schema?
    - How is a model selected at launch time and at runtime (CLI flags, ENV variables, config files, interactive slash commands, wire envelope)? What is the precedence between these mechanisms?
    - Can the CLI enumerate its model catalog programmatically (a `models`/`list` subcommand, an API, a config dump)?

- `## Sources`
    - add all useful resources that you used in your research as Markdown links

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLI's change is rapid and therefore you should assume that the prior research is out of date. You are reading this primarily to be able to effectively report the changes into the `## Changelog` section of the document. Critically, you should never substitute information
    in the old research for doing your own (up-to-date) research.

::end-block
- Perform research on topic
::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save research to `{{file}}`
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative contract, expressed as a `SimpleSchema`, and `md schema validate`
    > will enforce it against everything you write.

- Now we will capture other key metadata to the research documents Frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `has_official_schema` - set to "formal" if a formal schema exists for **model configuration**, "informal" if only an informal one exists, otherwise "none"
    - `schema_url` - if a formal or informal schema was discovered then set the URL for the schema's definition (always prefer formal over informal); otherwise do not add this property
    - `default_models` - one record per model available out of the box. `id` must be the **exact string** the CLI/config accepts; add `alias` when a short form exists, `context_window` when documented, and `is_default: true` on the model used when none is specified
    - `model_selection` - one record per mechanism for choosing a model (`cli_flag`, `env_var`, `config_file`, `interactive_command`, `wire_envelope`); `site` is the flag/variable/key/command name; give an `example` for each
    - `precedence` - the highest-wins ordering across the `model_selection` mechanisms (e.g. "cli_flag > env_var > config_file")
    - `custom_models` - one record per way to register bespoke/local models (`local`, `openai_compatible`, `anthropic_compatible`, `provider_plugin`, `other`); `config_site` names the config file/key involved
    - `dynamic_listing` - whether the CLI can enumerate its model catalog programmatically; if so name the `method` and give an `example`
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

1. all research in the body of the document 
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
