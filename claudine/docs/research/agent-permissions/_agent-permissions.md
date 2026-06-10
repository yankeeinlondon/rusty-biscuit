---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-permissions/{{state.file}}"
# all target documents we write to should provide this frontmatter
target_schema: 
    created: date
    last_updated: date(required)
    agent: string(required)
    model: string(required)
    # the CLI parameters involved in overriding permissions
    cli_params: "{ param: string, description: string, example: string, example_desc: string }[]"
    # the config files involved in setting permissions
    config_files: 
        user: string(required)
        repo: string(required)
    agent_permissions: 
        allowed: boolean(required)
    yolo:
        has_interactive_yolo: boolean(required)
        has_non_interactive_yolo: boolean(required)
    policy_engine:
        ergonomic: boolean(required)
        provides_coverage: boolean(required)
    changes: string[]
update: "{{file_exists(file) && markdown_file_empty(file)}}"
---

## Skills

Use the 'claudine' skill.

## Work Structure

Your job is to detailed research into the **permissions** features of the **{{state.desc}}**. You are expected to answer the following questions (and use the provided doc structure):

- `## Introduction to {{state.name}} Permissions` Section
    - Provide an overview of how permissions are defined in {{state.name}}
    - Describe how configuration files can define permissions
    - Describe any ENV variables that influence permissions
    - Describe the CLI parameters involved in permissions
        - what does each CLI switch do?
        - what precedence do the CLI parameters have versus ENV and config files
- `## Permissions Use Cases` Section
    - **Default**
        - If no ENV, Config files, or CLI switches provided any guidance permissions; what would the **default** permissions be for {{state.name}}
        - How would you use the `PolicyEngine` to describe this?
            - is the use of the PolicyEngine ergonomic for use with {{state.name}}?
            - are there features in PolicyEngine that would make use easier or more complete?
            - if no changes were made, would the PolicyEngine be able to define permissions for this use case? If no, then what prevents it?
    - **Whitelisting**
        - Describe how you would do the following with {{state.name}}:
            - set the "default permissions" to **no permissions** and then require that any _needed_ permissions be:
                - asked for in an interactive session
                - explicitly declared with the CLI or config files
        - Illustrate how additional permissions could be given via the CLI with a few concrete examples
        - How would you use the `PolicyEngine` to describe this?
            - is the use of the PolicyEngine ergonomic for use with {{state.name}}?
            - are there features in PolicyEngine that would make use easier or more complete?
            - if no changes were made, would the PolicyEngine be able to define permissions for this use case? If no, then what prevents it?
    - **YOLO**
        - what are the various ways a session can be put into YOLO mode
        - is YOLO mode available in interactive sessions? In non-interactive sessions?
        - when in YOLO mode what is allowed? What is not allowed?
    - **Root User**
        - when the CLI session is started as a root user does {{state.name}} behave differently with regard to permissions?
            - is YOLO still allowed?
    - **Configuring the Default**
        - what files are used to configure default permissions?
            - what file(s) for "user scope"?
            - what file(s) for "repo scope"?
            - give examples that illustrate the grammar which can be used to express permissions for {{state.name}}
    - **Extending the Base**
        - give a few practical examples of how default permissions might be set but then part of that policy is overwritten by 

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLI's change is rapid and therefore you should assume that the prior research is out of date. You are reading this primarily to be able to effectively report the changes into the `## Changelog` section of the document. Critically, you should never substitute information
    in the old research for doing your own (up-to-date) research.

::end-block
- Perform research on topic
    - take your time and make sure to be complete in your research
::block when="update"
- Update the document with your research
    - if you don't know something say that; don't make anything up and don't fall back to the old research as proof of anything
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save research to `{{file}}`
    - if you don't know something say that; don't make anything up
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
    - `cli_params` - set an array of dictionaries for each CLI property that {{state.name}} provides dealing with permissions. Each parameter will define:
        - `param` - the cli parameter (e.g., `yolo`)
        - `style` - set to one of the following:
            - `equals` when the format of the CLI is `yolo=true`
            - `switch` when the format of the CLI is `--yolo` or `--something <param>`
            - `positional` (note: rare that this would be used) if the parameter is not named but positional
        - `description` - a prose description of what this CLI parameter does
        - `example` - an example of using this parameter
        - `example_description` - describe the example you've provided
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document 
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
