---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-logging/{{state.file}}"
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: zai-coding-plan/glm-5.1
# all target documents we write to should provide this frontmatter
target_schema: 
    created: date
    last_updated: date(required)
    agent: string(required)
    model: string(required)
    has_schema: enum(formal,informal,none; required)
    schema_url: string
    logs_directory: { macos: string, windows: string, linux: string }
    log_format: enum(jsonl)
    has_desktop_app: boolean(required)
    desktop_logs: { same_log_format: boolean, same_directory: boolean }
    changes: string[]
    requires_claudine_update: boolean(required)
    reason: string
update: "{{file_exists(file) && markdown_file_empty(file)}}"
---

## Skills

Use the 'claudine' skill.

## Work Structure

Your job is to detailed research into the **logging** features of the **{{state.desc}}**. You are expected to answer the following questions:

- `## Introduction to {{state.name}} Logging` Section
    - An overview of log _locations_ for {{state.name}}
        - Go into details around how logs are organized, split, and archived
    - Where the logs are kept in storage:
        - JSONL
        - SQLite DB
        - etc.
    - Whether or not a SQLite (or other) database is used for storing logs
    - The major **types** of log messages that this provider distinguishes 

- `## Logging Schema` Section
    - Try to identify an "official" schema that {{state.name}} has defined for their log output
        - If found, document it's location and then convert this into a Rust struct/enum
    - If no "official" schema exists:
        - Document that no official schema exists
        - Look at any popular open sources projects which might have attempted to model a schema for the logs
        - Check if you have read access to actual log files on the host computer, if you do then analyze them for patterns
        - If neither community schemas exist nor do you have read access to the actual log files then state this
        - Otherwise, build a representative schema using Rust struct/enum's
- `## Informational Content versus Hook Events` Section
    - Claudine's current implementation for logging has been leveraging the hook event's that we can plug into rather than an Agent's actual log files
    - When do log files on the file system represent a better source? When do event logs represent a better source?
    - Are there any other sources which might help us enrich the data we're getting?

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
