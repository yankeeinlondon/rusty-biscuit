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
    last_updated: date(required)
    has_official_schema: boolean(required)
    schema_url: string
    logs_directory: { macos: string, windows: string, linux: string }
    log_format: enum(jsonl)
    has_desktop_app: boolean(required)
    desktop_logs: { same_log_format: boolean, same_directory: boolean }
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

## Idiomatic Markdown

::file @prompts/make-it-markdown.md

## Task

- Your research should be saved as `{{file}}`
- You should then write a schema definition to `{{file}}` in metadata:

    ```yaml
    $schema:
        last_updated: date(required)
        has_official_schema: boolean(required)
        schema_url: string
        logs_directory: { macos: string, windows: string, linux: string }
        log_format: enum(jsonl)
        has_desktop_app: boolean(required)
        desktop_logs: { same_log_format: boolean, same_directory: boolean }
    ```

- Once the document's body been saved to the filesystem, you'll set the following Frontmatter properties to the same document:
    - `last_updated` - today's date in YYYY-MM-DD format
    - `has_official_schema` - a boolean value indicating whether the schema was an "official" schema definition from the vendor.
        - if {{state.name}} has an official schema for logging then set `schema_url` to a URL reference to it
    - `logs_directory` - specify where the logs directory is typically located on a host (by operating system): { macos: string, windows: string, linux: string }
    - `has_desktop_app` - set as true/false based on whether the given provider not only has a CLI tool but also a desktop based application.
    - `desktop_logs` - as a dictionary:
        - `same_log_format` - set as a boolean value indicating whether the CLI and desktop apps write the same log format/schema or not
        - `same_directory` - set as a boolean value indicating whether the CLI
        and desktop apps share the same log file location or not

## Testing Done

You are done with the Markdown "{{file}}" has been saved with all research in the body of the document and the Frontmatter properties `last_updated` and `has_official_schema` set (optionally the `schema_url` property too).

- you do not need to run any tests or lints
- this task had no code modifications in it
