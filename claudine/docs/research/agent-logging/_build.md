---
sequence: "@claudine/docs/providers.yaml"
file: "claudine/docs/research/agent-logging/{{state.file}}"
grant:
    read:
        - "{{state.user_dir}}"
---

## Skills

Use the 'claudine' skill.

## Work Structure

Your job is to detailed research into the **logging** features of the **{{state.desc}}**. You are expected to answer the following questions:

- `## Introduction to {{state.name}} Logging` Section
    - An overview of log _locations_ for {{state.name}}
        - Go into details around how logs are organized, split, and archived
    - The format of the log files
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
- Once the document's body been saved to the filesystem, you'll set the following Frontmatter properties to the same document
    - `last_updated` - today's date in YYYY-MM-DD format
    - `has_official_schema` - a boolean value indicating whether the schema was an "official" schema definition from the vendor.
        - if {{state.name}} has an official schema for logging then set `schema_url` to a URL reference to it

## Testing Done

You are done with the Markdown "{{file}}" has been saved with all research in the body of the document and the Frontmatter properties `last_updated` and `has_official_schema` set (optionally the `schema_url` property too).

- you do not need to run any tests or lints
- this task had no code modifications in it


https://discord.com/api/webhooks/1487217707991564340/-zwkjV3aOliS2gpknG8BoSm1Oe0ur9BqgFTVwAUMomXefxzZxyRmQ7cRh2g16i2Ut2xM
