---
$schema:
    phase: number(required;default(1)) -> the phase of the plan to start with
    total_phases: number(required) -> the total number of phases the plan has
    plan: file(eager; required; match(**/*plan*.md)) -> the _plan file_ which is being implemented
    spec: file(eager; match(**/*spec*.md)) -> the _specification file_ which the plan was based on
    pass_icon: string
    commit_message: string -> if you pass in a git commit message then it will be used as the git message instead of using AI to calcuate it
    log: file -> the implementation's log file
description: |-
    Provide either a `plan` or `spec` filepath as a parameter and this
    prompt will detect the number of phases in the plan and then implement
    the project phase by phase.
plan: "{{ spec ? dirname(spec) + '/plan.md'  : null }}"
phase: "{{ file_exists(plan) ? frontmatter(plan, 'start_phase') || frontmatter(plan, 'phase') || 1 : null }}"
area: "{{ ctx.area ? ctx.area : ctx.is_monorepo ? 'monorepo-root' : 'repo-root' }}"
pass_icon: "{{ _loop_is_last ? '✅' : '🧑‍💻' }}"
total_phases: |-
    {{
        file_exists(plan)
            ? frontmatter(plan, 'total_phases') || frontmatter(plan, 'phases') || 0
            : 0
    }}
spec: |-
    {{
        file_exists(plan)
            ? file_exists(dirname(plan) + '/spec.md')
                ? dirname(plan) + '/spec.md'
                :  null
            : null
    }}
# The implementation's log file
log: "{{ dirname(spec || plan) + '/implementation-log.md' }}"
initialize:
    stack:
        - action:
            - ensure_file: "{{log}}"
        - when: "phase != 1"
          action:
              - set:
                    epilog: null
        - when: "!total_phases || total_phases <= 0"
          action:
              - warn: "The plan `{{plan}}` does not provide metedata on how many _phases_ the plan has!"
              - message: "The plan `{{plan}}` does not provide metedata on how many _phases_ the plan has!"
              - error: "for a plan to be implemented using the **implement-plan** prompt, you need to ensure the plan ({{plan}}) has set either `total_phases` or `phases` Frontmatter property!"
        - when: "file_exists(log) && frontmatter(log, 'message_to_agent')"
          action:
              - stderr: |-
                    The previous phase's agent has passed a message to this agent:

                    > {{frontmatter(log, 'message_to_agent')}}
              - message: |-
                    The previous phase's agent has passed a message to this agent:
        
                    > {{frontmatter(log, 'message_to_agent')}}
              - set:
                    epilog: "{{message_to_agent}}"
                    message_to_agent: null

                  
                  
start:
    message: "🎬  implementing phase **#{{phase}}** of `{{parent_dir(plan)}}` (**area:** {{ ctx.area || ctx.repo }}, **agent:** {{ctx.agent}}/{{ctx.model}})"
success: 
    say: "Phase {{phase}} of the plan in the {{area}} {{ctx.area_description}}, was implemented successfully"
    message: "{{pass_icon}}  phase **{{phase}}** (_of {{total_phases}}_) of the plan `{{parent_dir(plan)}}` successfully completed ({{area}}, {{ctx.agent}}/{{ctx.model}})"
    success: "Completed the implementation of <b>phase <yellow>{{phase}}</yellow></b> of the {{link(plan)}} plan"
    stack:
        - when: "ctx.dirty_files && !commit_message"
          action:
            - message: |-
                staging all files from phase **#{{phase}}** in preparation for the git commit:

                {{ as_unordered_list(ctx.dirty_files) }}
            - shell: git add ..
            - shell: just commit
        - when: "ctx.dirty_files && commit_message"
          action:
            - message: |-
                staging all files from phase **#{{phase}}** in preparation for the git commit:

                {{ as_unordered_list(ctx.dirty_files) }}
            - shell: git add ..
            - shell: git commit -m "{{commit_message}}"
        - when: "!ctx.dirty_files"
          action:
              - message: phase {{phase}} of the plan made no file changes!
              - warn: phase {{phase}} of the plan made no file changes!
blocked:
    message: "💥  phase **{{phase}}** (_of {{total_phases}}_) was **blocked** because it has shell commands which were not approved for execution!"
failure:
    say: "Phase {{phase}} of a plan in the {{area}} package area, ran into problems!"
    message: "❌️  phase **{{phase}}** (_of {{total_phases}}_) failed in the plan `{{parent_dir(plan)}}` ({{area}}, {{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
    effect: sad-trombone
loop:
    until: "phase >= total_phases || frontmatter(log, 'human_involvement')"
    action: "increment(phase)"
---
::block when="total_phases"
# Implement Phase {{phase}} of {{total_phases}}
::end-block
::block when="!total_phases"
# Implement Phase {{phase}}

> ⚠️ there was no `total_phases` set on this plan! This metadata missing may indicate a problem
::end-block

::file "../_no_formatting.md"

Your task is to implement phase {{phase}} of the plan found in '@{{area}}/{{plan}}'.

- check off tasks in the plan -- marked by GFM todos (aka., `[ ]`) -- once they are complete
    - don't wait until the end of the phase
    - marking tasks complete in real time allows graceful recovery of the implementation of the plan if anything were to go wrong with the initial implementation of the plan

::block when="spec"
> **NOTE:** this plan is based on the specification file: {{spec}}
::end-block

## Logging

You will log your implementation progress to: {{log}}

::block when="!file_exists(log) || (markdown_body_empty(log) && is_empty(frontmatter(log)))"
- the log file for this implementation has not been started yet
- make sure the log file ({{log}}) exists -- it may already exist as an empty file -- and then write to it
- add the following sections:
    - `# Implementation Log for {{parent_dir(spec)}} ({{total_phases}} phases)` for the logs title
    - `## Phase {{phase}}` for the log entries of this phase
- then add the following metadata:
    - set `spec` to "{{spec}}"
    - set `plan` to "{{plan}}"
    - set `implemented_by` to "{{ctx.agent}}/{{ctx.model}}"
    - set `started_phase` to "{{phase}}"
::end-block

Now we need to update the specification file's frontmatter ({{spec}}):

- set `implemented` to `true`
- set `implemented_by` to "{{ctx.agent}}/{{ctx.model}}"

::block when="file_exists(log) && (!markdown_body_empty(log) || !is_empty(frontmatter(log)))"
- the log file already has content from earlier work; keep it and append to it
- you'll need to add a new H2 section `## Phase {{phase}}` to the document for log entries during this phase of the implementation
::block when="phase > 1"
- since we are implementing phase {{phase}}, we do have the log entries for {{phase - 1}} which you can review at: {{log}}
::end-block
::end-block

## Test Design Requirements

Before changing implementation code, map every behavior changed by this phase
to a concrete test. For regressions, first add or identify a test that fails
for the reported behavior and succeeds only after the fix.

For every changed behavior:

- test the public observable result, not implementation details
- include the original failing input exactly
- include relevant representation variants, such as native versus quoted
YAML values, missing versus present values, and boundary values
- assert dependent outputs and downstream state, not only the immediate value
- include negative/error behavior where malformed or invalid input is possible
- choose the verification level required by `rust-testing`; unit coverage alone
is insufficient when behavior crosses crate, CLI, filesystem, terminal, or
persistence boundaries
- log important details of your work to {{log}} under the `## Phase {{phase}}` section

When changing parsers, schemas, templates, prompts, or configuration-driven
behavior:

- add a passive corpus test covering all shipped artifacts
- add at least one end-to-end test using the real shipped artifact and normal
invocation path
- include a repeated read/write/read round trip when values are persisted

A broad test suite passing does not substitute for a targeted regression test.
Before declaring the phase complete, report the requirement-to-test mapping,
the exact targeted tests added, the broader gates run, and every skipped or
pre-existing failure.

## Completion

You are done when:

- all functionality defined in phase {{phase}} has been implemented
- all tests are passing (using `just test` in the {{ctx.current_package_area}} package area)
- all tests meet the design requirements for testing (stated above)
- all lints are passing (using `just lint` in the {{ctx.current_package_area}} package area)
- all GFM tasks/todos in the plan have been completed (and have been marked as complete)
    - NOTE: you should mark tasks as complete as soon as you believe they are complete (e.g., implemented and any relevant tests suggest this is complete). Doing this allows an immediate feedback loop but also helps in recovering from a phase that didn't complete
- You must set the following Frontmatter properties to both the plan file ({{plan}}) and the implementation log ({{log}}):
    - `source_files_during_phase_{{phase}}` should be set to all source code files which were created or updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_updated_during_phase_{{phase}}` should be set to all documentation files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_created_during_phase_{{phase}}` should be set to all documentation files which were created during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `skills_files_updated_during_phase_{{phase}}` should be set to all agent skill files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    ::block when="phase == total_phases"
        - set `source_code` Frontmatter to every source code file that was updated or created during the various phases of the plan
        - set `documentation` Frontmatter to every documentation file that was updated or created during the various phases of the plan
        - set `completed_phase` to "{{phase}}"
        - set `implemented` to `true`
    ::end-block
    ::block when="ctx.is_monorepo"
    - set `packages` as a list of packages in the monorepo which were touched by the implementation in phase {{phase}}
    ::end-block
- If something happened during the implementation of this phase (# {{phase}}) that you feel requires human involvement before proceeding onto the next phase for implementation then set `human_review` to `true` (otherwise set to `false`)
    - this should be kept to a minimum as it will impact flow but when it's necessary it needs to be raised
    - when `human_review` is set to `true` you must also set `human_review_items` as a list of things which the human is expected to weigh in on and why this decision is critical to be made before the next phase is implemented
- if something happened during the implementation that you think is important to know for the agent who will implement the next phase then set the `message_to_agent` property with that message and it will be passed to the agent; be sure you've provided enough context that what you're communicating is clear.

**IMPORTANT:** 

::block when="ctx.area"
- use the '{{area}}' skill during the implementation
::block when="phase == total_phases"
- do NOT move the spec directory into the `_completed` folder when the final phase is complete (that is done as a separate step which you are not responsible for)
::end-block
::end-block
- Do NOT commit or stage files to git, this will be done as a separate process.
- Report a summary of what you did including all the source files you changed.
- You do not need to run tests across the entire monorepo as this will take far too long. Only 
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!

::file "../_os.md"
