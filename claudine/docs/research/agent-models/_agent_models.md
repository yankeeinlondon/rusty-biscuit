---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-models/{{state.file}}"
agent: opencode
model: zai-coding-plan/glm-5.2
# all target documents we write to should provide this frontmatter
target_schema: 
    created: date
    last_updated: date(required)
    agent: string(required)
    model: string(required)
    
    changes: string[]
    requires_claudine_update: boolean(required)
    reason: string
update: "{{file_exists(file) && markdown_file_empty(file) ? false : true }}"
---

## Skills

Use the 'claudine' skill.

## Document Structure

Your job is to do detailed research into the **model** support in the **{{state.desc}}** solution. You are expected to answer the following questions:

- `## Model's Available` Section
    - what models are available by default when you install **{{state.name}}**?
    - how can you add in bespoke models (local models or otherwise)?

- `## Model Configuration Details` Section

    - Does **{{state.name}}** provide a formal schema for the configuration of it's models? An informal schema?
    - Is 


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
- Set the `$schema` property of `{{file}}` to:

    {{target_schema}}

    > Note: this is using the `SimpleSchema` schema representation which can be easily converted to JSON schema for validation purposes

- Now we will capture other key metadata to the research documents Frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `has_official_schema` - set to "formal" if a formal schema exists, set to "informal" if an informal schema exists, otherwise set to "none"
    - `schema_url` - if there is a formal or informal schema discovered then set the URL for the schemas definition (always prefer formal over informal); otherwise do not add this property
    - `logs_directory` - specify where the base of the logs directory is typically located on a host (by operating system): { macos: string, windows: string, linux: string }
    - `has_desktop_app` - set as true/false based on whether the given provider not only has a CLI tool but also a desktop based application.
    - `desktop_logs` - as a dictionary:
        - `same_log_format` - set as a boolean value indicating whether the CLI and desktop apps write the same log format/schema or not
        - `same_directory` - set as a boolean value indicating whether the CLI
        and desktop apps share the same log file location or not
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
