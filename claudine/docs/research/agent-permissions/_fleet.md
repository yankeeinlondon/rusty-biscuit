---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-permissions/{{state.file}}"
# NOTE: `grant:` is not implemented yet — until it is, run this sequence with
# `--yolo` so the provider can Read files under {{state.user_dir}}; without it
# OpenCode's external_directory permission is auto-rejected in non-interactive
# mode and the research agent stops prematurely.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
# the frontmatter contract for target documents lives in the schema sidecar
# (./_schema.yaml) so the contract is single-sourced and machine-validated
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
# make interrupted fleet runs resumable: skip providers already researched today
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **Agent Permissions**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **Agent Permissions** that is current; skipping updates"
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
              - info: "The **Agent Permissions** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Agent Permissions** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent Permissions research on **{{state.name}}** failed to complete!"
    warn: "The Agent Permissions research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---

## Skills

Use the 'claudine' skill.

## Document Structure

Your job is to research the **permissions and security-control** features of
**{{state.desc}}** for Claudine's `PolicyEngine` and provider metadata catalog. The
legacy `permissions/` topic has been merged into this topic, so answer both the high-level
permission questions and the lower-level rule/sandbox/trust/MCP questions that affect
whether Claudine can model or mutate the provider's policy safely.

Do not treat legacy research as current truth. If this is an update run, read the existing
document and use it only to report changes and avoid losing useful topics; verify all
facts against current docs, source, and observed config files.

Use this document structure:

- `## Introduction to {{state.name}} Permissions` Section
    - Provide an overview of how permissions are defined in {{state.name}}
    - Describe how configuration files can define permissions
    - Describe any ENV variables that influence permissions
    - Describe the CLI parameters involved in permissions
        - what does each CLI switch do?
        - what precedence do the CLI parameters have versus ENV and config files
    - Distinguish permission/approval policy from tool visibility. For example, a provider
      may separately decide which tools are visible to the model and which visible tools
      are pre-approved.
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
        - Identify the best **CLI-only, session-scoped** invocation for starting with
          no permissions or no tools. This is for a future Claudine wrapper feature:
          Claudine should be able to launch the provider in a locked-down posture and
          then explicitly add back only the permissions the user requested, without
          mutating the user's provider config or affecting provider runs outside
          Claudine.
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
    - Describe how our permissions map to tool calls
    - Capture the provider's native permission entities: tool, tool group, command,
      filesystem path, workspace boundary, MCP server, MCP tool/resource, subagent/agent,
      mode, approval category, sandbox, hook, extension, slash command, or other
    - Document the provider's rule grammar, including decisions (`allow`, `ask`, `deny`,
      `prompt`, `forbidden`, etc.), matcher syntax, wildcard/glob/regex semantics,
      merge behavior, and conflict precedence
    - Document approval modes and aliases, especially partial modes such as plan,
      auto-edit, accept-edits, auto/classifier modes, and provider-specific names
    - Document whether approvals persist, and if so whether they persist for a session,
      project, command pattern, path, or permanently
- `## Sandboxing, Trust, and Administrative Controls`
    - Describe any sandbox mode separately from approval mode: OS backend, filesystem
      read/write scope, network controls, process isolation, known platform differences,
      and failure behavior when sandboxing is unavailable
    - Describe folder/project trust and whether trust gates project config, memory,
      extensions, hooks, MCP servers, custom commands, or auto-approval
    - Describe managed/admin policy layers and whether they replace, merge, or constrain
      user/repo settings
    - List protected paths or provider-reserved files that remain guarded even under
      permissive modes
    - State the security posture honestly: is the permission system an OS-enforced
      sandbox, an advisory UX guardrail, a static policy engine, or a combination?
- `## MCP and Permissions`
    - Describe how permissions and MCP interact
    - How can you use permissions to make MCP safer?
    - Document server-level filters, tool-level filters, trust flags, resource access,
      response interception/sanitization, and whether MCP tools run inside or outside
      the provider's sandbox
- `## Non-Interactive Behavior`
    - Explain how permission prompts behave in headless/print/exec/run modes
    - State whether the provider has a programmatic approval channel, excludes
      approval-required tools, auto-approves, fails, or can hang when approval is needed
- `## Sources`
    - Add all useful resources that you used in your research as Markdown links

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLI's change is rapid and therefore you should assume that the prior research is out of date. You are reading this primarily to be able to effectively report the changes into the `## Changelog` section of the document. Critically, you should never substitute information
    in the old research for doing your own (up-to-date) research.

::end-block
- Perform research on topic
    - take your time and make sure to be complete in your research
    - inspect actual config files under `{{state.user_dir || 'the provider user config directory'}}`
      when that directory is known and exists; prefer observed current config shapes over
      stale documentation, and state when no local config exists to inspect
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
        - `description` - a prose description of what this CLI parameter does;
          when the param takes a value, include the value shape here (e.g.
          "takes a comma-separated rule list")
        - `example` - an example of using this parameter
        - `example_description` - describe the example you've provided
        - Include adjacent security-control switches here too: tool include/exclude
          flags, extension/profile selectors, sandbox/container flags, approval-mode
          flags, no-tools/read-only flags, MCP/trust switches, and non-interactive
          permission-prompt switches. Do not return `[]` merely because the provider
          lacks a direct `--mode`/`--yolo` flag; return `[]` only after checking
          `--help`, subcommand help, and docs for these adjacent surfaces, and explain
          that absence in the body.
    - `env_vars` - one record per environment variable that influences permissions:
        - `name` - the environment variable name
        - `effect` - prose description; this stays authoritative for specifics such
          as value sets, version gates, precedence quirks, and hardening direction
        - `effect_category` - exactly one label per variable, chosen from:
          `sandbox_control`, `none`, `state_home_relocation`, `config_path_override`,
          `tool_surface`, `security_hardening`, `customization_lockdown`,
          `credential`, `threat_detection`, `policy_overlay`, `config_injection`,
          `config_source_toggle`, `approval_mode`, `network_control`,
          `workspace_trust`, `other`. `none` is a first-class answer for verified
          non-permission variables — keep those records rather than pruning them;
          use `other` only when nothing else fits
    - `config_files` - an array of dictionaries describing where permission
      configuration can live. File paths must be recorded separately for macOS, Linux,
      and Windows; do not use `os: all` for path fields.
        - `os` - one of `macos`, `linux`, or `windows`
        - `user` - the relative (typically relative to user's home dir) filepath to the configuration file used for user scoped permissions
        - `repo` - the relative (from repo root) filepath to the configuration file used for repo scoped permissions
        - `notes` - OS-specific path or config-scope caveats
    - `precedence` - ordered highest-to-lowest policy source precedence. Do not add
      a numeric rank; the array order is the rank. Use multiple records when one
      source has different behavior for different scopes:
        - `source` - provider-native source name such as `cli`, `env`, `repo_config`,
          `user_config`, `managed_policy`, or the provider's exact term
        - `scope` - string array of affected policy surfaces, using only this
          vocabulary: `rules`, `mcp`, `approval_mode`, `sandbox`,
          `tool_visibility`, `general_config`, `config_loading`, `agents`,
          `extensions`, `hooks`, `skills`, `slash_commands`,
          `customization_resources`, `security_controls`, `trust`, `workspace`,
          `provider_model`, `other`. Do not invent new scope tokens; put
          anything the vocabulary cannot express in `notes`.
        - `merge_strategy` - `none` for replacement/override, `shallow` for
          top-level merge, `deep` for nested merge, or `nearest` when closest or
          most-specific scope wins inside that source family
        - `notes` - caveats such as trust gates, safe mode disabling a source,
          admin policy constraining rather than overriding, or rule conflicts like
          deny-wins
    - `default_posture` - a one-to-two sentence summary of the effective permissions when nothing is configured
    - `cli_zero_permissions` - the CLI-only, session-scoped way to start with no
      permissions or no tools:
        - `supported` - true if {{state.name}} can be launched from the CLI in a
          no-permissions/no-tools baseline without mutating provider config
        - `invocation` - the complete CLI invocation or flag set Claudine could use
        - `mechanism` - which provider mechanism this uses, such as empty tool
          allowlist, restrictive approval mode, deny-all rule, read-only sandbox, or
          provider-native no-tools flag
        - `limitations` - what remains allowed, what cannot be disabled by CLI, and
          whether additional permissions can be added back via CLI in the same run
    - `agent_permissions`
        - `allowed` - set to true/false based on whether {{state.name}} allows permissions to be set on an agent/subagent scoped basis
        - `fm_properties` - a string array of the frontmatter/config properties involved in agent-scoped permissions (omit when `allowed` is false)
    - `yolo` - `has_interactive_yolo` / `has_non_interactive_yolo` booleans plus `mechanism` naming the flag/env/config switch used
    - `policy_engine` - `ergonomic` / `provides_coverage` booleans plus a `gaps` string array listing anything the PolicyEngine cannot express for {{state.name}}
    - `permission_entities` - one record per native entity the provider can target with
      permissions or adjacent security controls:
        - `entity` - one of `tool`, `tool_group`, `command`, `path`, `workspace`,
          `mcp_server`, `mcp_tool`, `mcp_resource`, `agent`, `subagent`, `mode`,
          `approval_category`, `sandbox`, `hook`, `extension`, `slash_command`, or
          `unknown`
        - `native_names` - provider-native names or config keys for that entity
        - `notes` - how the entity is evaluated or why it matters
    - `approval_modes` - one record per coarse session mode:
        - `name` - provider-native mode name
        - `effect` - what it changes
        - `interactive` - whether it is available in interactive sessions
        - `non_interactive` - whether it is available in non-interactive sessions
        - `aliases` - CLI/config/env/slash-command aliases
    - `rule_model` - structured summary of fine-grained rules:
        - `decisions` - provider-native decision values
        - `syntax` - compact description of rule syntax
        - `precedence` - conflict ordering (for example deny > ask > allow)
        - `merge_semantics` - how scopes combine, override, intersect, or deep-merge
        - `matcher_semantics` - glob/regex/prefix/path/shell-pattern details
        - `default_decision` - what happens when no rule matches
    - `tool_visibility` - whether the provider separately supports hiding/restricting
      tools independent of approval, with `mechanisms` and `notes`
    - `sandbox` - whether the provider has sandboxing, including supported `modes`,
      OS/container `backends`, `filesystem_control`, `network_control`, and `notes`
    - `trust_and_admin` - summarize folder/project trust, managed/admin policy, safe mode,
      and any notes about precedence or disabled project-local surfaces
    - `mcp_permissions` - whether MCP permissions are supported, with server filters,
      tool filters, trust model, and notes about resources, response interception, or
      sandbox bypass
    - `headless_behavior` - one-to-two sentences about permissions in non-interactive
      modes
    - `approval_persistence` - one-to-two sentences about session/project/permanent
      approval persistence
    - `protected_paths` - list provider-reserved paths or patterns that are specially
      protected
    - `security_posture` - one-to-two sentences classifying the provider's controls as
      OS-enforced sandbox, advisory guardrail, static policy, managed policy, etc.
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
