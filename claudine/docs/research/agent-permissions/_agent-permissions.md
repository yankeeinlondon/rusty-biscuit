---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-permissions/{{state.file}}"
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
        - give a few practical examples of how default permissions might be set but then part of that policy is overwritten by a narrower scope (repo config overriding user config, or CLI switches overriding both)
- `## Tools and Permissions`
    - List out the tools that {{state.name}} provides by default
    - Describe out permissions map to tool calls
- `## MCP and Permissions`
    - Describe how permissions and MCP interact
    - How can you use permissions to make MCP safer?

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
    - `cli_params` - set an array of dictionaries for each CLI property that {{state.name}} provides dealing with permissions. Each parameter will define:
        - `param` - the cli parameter (e.g., `yolo`)
        - `style` - set to one of the following:
            - `equals` when the format of the CLI is `yolo=true`
            - `switch` when the format of the CLI is `--yolo` or `--something <param>`
            - `positional` (note: rare that this would be used) if the parameter is not named but positional
        - `description` - a prose description of what this CLI parameter does
        - `example` - an example of using this parameter
        - `example_description` - describe the example you've provided
    - `env_vars` - one record per environment variable that influences permissions: `name` and its `effect`
    - `config_files` - is a dictionary and you must set both properties:
        - `user` - the relative (typically relative to user's home dir) filepath to the configuration file used for user scoped permissions
        - `repo` - the relative (from repo root) filepath to the configuration file used for repo scoped permissions
    - `precedence` - the highest-wins ordering across CLI params, env vars, and config files (e.g. "cli > env > repo config > user config")
    - `default_posture` - a one-to-two sentence summary of the effective permissions when nothing is configured
    - `agent_permissions`
        - `allowed` - set to true/false based on whether {{state.name}} allows permissions to be set on an agent/subagent scoped basis
        - `fm_properties` - a string array of the frontmatter/config properties involved in agent-scoped permissions (omit when `allowed` is false)
    - `yolo` - `has_interactive_yolo` / `has_non_interactive_yolo` booleans plus `mechanism` naming the flag/env/config switch used
    - `policy_engine` - `ergonomic` / `provides_coverage` booleans plus a `gaps` string array listing anything the PolicyEngine cannot express for {{state.name}}
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
