---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-logging/{{state.file}}"
# NOTE: `grant:` is not implemented yet — until it is, run this sequence with
# `--yolo` so the provider can Read files under {{state.user_dir}}; without it
# OpenCode's external_directory permission is auto-rejected in non-interactive
# mode and the research agent stops prematurely.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: zai-coding-plan/glm-5.2
# the frontmatter contract for target documents lives in the schema sidecar
# (./_schema.yaml) so the contract is single-sourced and machine-validated
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
# make interrupted fleet runs resumable: skip providers already researched today
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **Agent Logging**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **Agent Logging** that is current; skipping updates"
              - skip
# a provider exiting 0 is not proof the research was written — verify the
# agent actually stamped today's date before accepting success
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Agent Logging** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Agent Logging** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent Logging research on **{{state.name}}** failed to complete!"
    warn: "The Agent Logging research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Agent Logging Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

This topic covers the **log surfaces**, **record semantics**, and **time semantics** of {{state.name}}: where logs live on disk, how they are organized and archived, what record types they contain, and how their timestamps behave. It feeds Claudine's log ingestion and observability layers. Boundaries against sibling topics: the hooks topic owns lifecycle-event semantics, and the usage topic owns quota inspection — mention those surfaces here only where they explain a log record.

## Document Structure

Your job is to do detailed research into the **logging** features of the **{{state.desc}}**. You are expected to answer the following questions:

- `## Introduction to {{state.name}} Logging` Section
    - An overview of log _locations_ for {{state.name}}
        - Go into details around how logs are organized, split, and archived
    - Where the logs are kept in storage:
        - JSONL
        - SQLite DB
        - etc.
    - Whether or not a SQLite (or other) database is used for storing logs
    - The major **types** of log messages that this provider distinguishes
    - Any environment variables that relocate the log directories or alter what gets logged (config-dir overrides, verbosity/telemetry toggles, log-level settings)

- `## Logging Schema` Section
    - Try to identify an "official" schema that {{state.name}} has defined for their log output
        - If found, document its location and then convert this into a Rust struct/enum
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
    - **Evidence requirement:** you have read access to `{{state.user_dir}}` on this host. Inspect the *actual* log files/directories there and prefer what you observe over what documentation claims (`confidence: observed` beats `documented`). Real logs regularly contain surfaces, record types, and time formats the documentation omits.
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
    - `has_official_schema` - set to "formal" if a formal schema exists, set to "informal" if an informal schema exists, otherwise set to "none"
    - `schema_url` - if there is a formal or informal schema discovered then set the URL for the schemas definition (always prefer formal over informal); otherwise do not add this property
    - `surfaces` - one record per **log surface** the provider writes (session transcripts, subagent transcripts, session/history indexes, prompt history, application logs, state databases, live metadata, statusline). For each record fill the fields defined in `_schema.yaml`:
        - `path_*` values are **templates**, not literal paths — use placeholders like `{session_id}`, `{sanitized_cwd}`, `{pid}` and keep date-sharding visible (e.g. `sessions/YYYY/MM/DD/...`)
        - `live_locked` - set true for any surface with live lock/WAL files (e.g. SQLite in WAL mode); these must never be copied or symlinked while the app runs
        - `schema_versioning` - how the surface signals schema changes: an explicit version field in the data (`explicit_field`), a version suffix in the filename such as `logs_2.sqlite` (`filename_suffix`), or `none`
    - `time_fields` - one record per **timestamp site** across the surfaces (including timestamps embedded in *filenames*). For each: the `site` (JSONPath-ish location or `filename`), the `unit` (`iso8601`/`unix_seconds`/`unix_millis`), the `zone` (`utc`/`local`/`embedded_offset`/`unspecified`), and your `confidence` (`source_code` > `observed` > `documented` > `inferred`)
        - **you must answer unit and zone for every site** — if you cannot establish the zone, record `unspecified` with `confidence: inferred`; never omit the record
    - `record_types` - one record per structured surface: the discriminator field (e.g. `type`) and the **observed** vocabulary of its values
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
