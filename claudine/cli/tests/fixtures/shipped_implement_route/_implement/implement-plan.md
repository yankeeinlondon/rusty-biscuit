---
description: |-
    Implements a plan phase by phase.

    - if an appropriately named _specification_ or _design_ file is found in the same directory then they will be referenced to in the log's frontmatter as well as the prose content 
$schema:
    phase: number(required;default(1)) -> the phase of the plan to start with
    total_phases: number(required) -> the total number of phases the plan has
    plan: file(eager; required; match(**/*plan*.md)) -> the _plan file_ which is being implemented
    spec: file(eager; match(**/*spec*.md)) -> the _specification file_ which the plan was based on
    pass_icon: string
    commit_message: string -> if you pass in a git commit message then it will be used as the git message instead of using AI to calcuate it
    log: file -> the implementation's log file
    in_loop: boolean -> set by the `review-loop` sequence; when true this step opts out once the spec is marked `implemented`
yolo: true
commit_message: ""
plan: "{{ spec ? dirname(spec) + '/plan.md'  : null }}"
phase: "{{ file_exists(plan) ? frontmatter(plan, 'start_phase') || frontmatter(plan, 'phase') || 1 : null }}"
area: "{{ ctx.area ? ctx.area : ctx.is_monorepo ? 'monorepo-root' : 'repo-root' }}"
pass_icon: "{{ loop.is_last ? '✅' : '🧑‍💻' }}"
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
    # Inside the review-loop sequence the step list is static; a plan whose
    # spec is already implemented opts out here and launches nothing.
    - when: "in_loop && spec && frontmatter(spec, 'implemented') == true"
      then:
          - message: "implementation skipped: `{{parent_dir(spec)}}` is already marked implemented"
          - skip
    - ensure_file: "{{log}}"
    - when: "!total_phases || total_phases <= 0"
      then:
          - warn: "The plan `{{plan}}` does not provide metedata on how many _phases_ the plan has!"
          - message: "The plan `{{plan}}` does not provide metedata on how many _phases_ the plan has!"
          - error: "for a plan to be implemented using the **implement-plan** prompt, you need to ensure the plan ({{plan}}) has set either `total_phases` or `phases` Frontmatter property!"

start:
    - message: "🎬  implementing phase **#{{phase}}** of `{{parent_dir(plan)}}` (**area:** {{ ctx.area || ctx.repo }}, **agent:** {{ctx.agent}}/{{ctx.model}})"
    # `start` (not `initialize`) because a loop re-enters each phase here; the
    # body renders the same message for the agent itself.
    - when: "file_exists(log) && frontmatter(log, 'message_to_agent')"
      then:
          - message: |-
                The previous phase's agent has passed a message to this agent:

                {{file_exists(log) ? frontmatter(log, 'message_to_agent') : null}}
success: 
    - success: "Completed the implementation of <b>phase <yellow>{{phase}}</yellow></b> of the {{link(plan)}} plan"
    - message: "{{pass_icon}}  phase **{{phase}}** (_of {{total_phases}}_) of the plan `{{parent_dir(plan)}}` successfully completed ({{area}}, {{ctx.agent}}/{{ctx.model}})"
    - when: "ctx.dirty_files && !commit_message"
      then:
        - message: |-
            staging dirty files [{{ length(ctx.dirty_files) }}] from phase **#{{phase}}** in preparation for the git commit:

            {{ as_unordered_list(ctx.dirty_files) }}
    - when: "ctx.dirty_files && commit_message"
      then:
        - message: |-
            staging all files from phase **#{{phase}}** in preparation for the git commit:

            {{ as_unordered_list(ctx.dirty_files) }}
    - when: "!ctx.dirty_files"
      then:
          - message: phase {{phase}} of the plan made no file changes!
          - warn: phase {{phase}} of the plan made no file changes!
blocked:
    message: "💥  phase **{{phase}}** (_of {{total_phases}}_) was **blocked** because it has shell commands which were not approved for execution!"
failure:
    message: "❌️  phase **{{phase}}** (_of {{total_phases}}_) failed in the plan `{{parent_dir(plan)}}` ({{area}}, {{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
loop:
    until: "phase >= total_phases || frontmatter(log, 'human_review') == true"
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

::file "../_frontmatter_edits.md"

Your task is to implement phase {{phase}} of the plan found in '@{{area}}/{{plan}}'.

- check off tasks in the plan -- marked by GFM todos (aka., `[ ]`) -- once they are complete
    - don't wait until the end of the phase
    - marking tasks complete in real time allows graceful recovery of the implementation of the plan if anything were to go wrong with the initial implementation of the plan

::block when="spec"
> **NOTE:** this plan is based on the specification file: {{spec}}
::end-block

::file "../_headless-orchestration.md"

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

::block when="file_exists(log) && (!markdown_body_empty(log) || !is_empty(frontmatter(log)))"
- the log file already has content from earlier work; keep it and append to it
- you'll need to add a new H2 section `## Phase {{phase}}` to the document for log entries during this phase of the implementation
::block when="phase > 1"
- since we are implementing phase {{phase}}, we do have the log entries for {{phase - 1}} which you can review at: {{log}}
::end-block
::end-block

::block when="file_exists(log) && frontmatter(log, 'message_to_agent')"
### Message from the Previous Phase's Agent

The agent that implemented the previous phase left this message for you:

{{file_exists(log) ? frontmatter(log, 'message_to_agent') : null}}
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

::file "../_input-robustness.md"

A broad test suite passing does not substitute for a targeted regression test.
Before declaring the phase complete, report the requirement-to-test mapping,
the exact targeted tests added, the broader gates run, and every skipped or
pre-existing failure.

::file "../_test-tiers.md"

## Completion

You are done when:

- all functionality defined in phase {{phase}} has been implemented
- all tests are passing (using `just test` in the {{ctx.current_package_area}} package area)
- all tests meet the design requirements for testing (stated above)
- all lints are passing (using `just lint` in the {{ctx.current_package_area}} package area)
- all GFM tasks/todos in the plan have been completed (and have been marked as complete)
    - NOTE: you should mark tasks as complete as soon as you believe they are complete (e.g., implemented and any relevant tests suggest this is complete). Doing this allows an immediate feedback loop but also helps in recovering from a phase that didn't complete

### When the Test and Lint Gates May Be Skipped

`just test` and `just lint` are required unless the CI scope calculator shows
that this phase changed nothing any package reads. You do not decide that by
judgment; you decide it by running this from the repository root after all of
the phase's edits (including plan, log, and spec frontmatter) are saved:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

The uncommitted working tree is exactly this phase's changes, because each
earlier phase is committed when it succeeds.

- if the output has `"packages": []`, the gates may be skipped
    - record the command and its `packages` / `change_class` values in the log
      under `## Phase {{phase}}`, and state in your final summary that the
      gates were skipped and why
- in every other case run both gates, including when:
    - `packages` is non-empty: a "documentation" file such as a Markdown
      fixture, golden file, or `include_str!` target counts as source when
      compiled code reads it, and the calculator knows that while you may not
    - the command errors or its output cannot be read
    - you listed any file in `source_files_during_phase_{{phase}}`
- never skip a gate because the change is small, "only a rename", "only
  comments", or "only tests"; those all compile

### Plan and Log Frontmatter

- You must set the following Frontmatter properties to both the plan file ({{plan}}) and the implementation log ({{log}}):
    - `source_files_during_phase_{{phase}}` should be set to all source code files which were created or updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_updated_during_phase_{{phase}}` should be set to all documentation files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_created_during_phase_{{phase}}` should be set to all documentation files which were created during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `skills_files_updated_during_phase_{{phase}}` should be set to all agent skill files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    ::block when="ctx.is_monorepo"
    - set `packages` as a list of packages in the monorepo which were touched by the implementation in phase {{phase}}
        ::block when="phase > 1"
        - this is _not_ the first phase so be sure to _add_ the packages which were touched in this phase
        - the end goal when the plan is _fully_ implemented is to have this property represent the packages touched throughout all phases of the implementation
        ::end-block
    ::end-block

### Handoff to the Next Phase

Set these properties on the implementation log ({{log}}), never on the spec or plan:

- If something happened during the implementation of this phase (# {{phase}}) that you feel requires human involvement _before_ proceeding onto the next phase for implementation then set `human_review` to `true` (otherwise set to `false`)
    - this should be kept to a minimum as it will impact flow but when it's necessary it needs to be raised
    - when `human_review` is set to `true` you must also set `human_review_items`:
        - a list of things which the human is expected to weigh in on
        - each item should use the YAML `|-` operator so that the text description is more easily read by a human
        - don't use jargon and don't assume the human knows this repo or the details of this plan
        - each list item should describe:
            - why this decision is critical to be made _before_ the next phase is implemented
            - bullet point out various options/solutions to choose from
            - each option/solution should have a clear description and relative pros and cons
            - you must recommend one of these options and explain why you're recommending it
        - you can use Markdown formatting to help produce a nice looking and clear message
- if something happened during the implementation that you think is important to know for the agent who will implement the next phase then set the `message_to_agent` property with that message and it will be passed to the agent; be sure you've provided enough context that what you're communicating is clear.
    - `message_to_agent` holds only your message: replace any message left by an earlier phase, and remove the property if you have nothing to pass on

::block when="spec"
### Spec Frontmatter

The following Frontmatter properties must be set on the spec file ({{spec}}):

::block when="!frontmatter(spec, '$schema')"
::file ../_set_spec_schema.md 
::end-block
::end-block

::block when="phase == total_phases"
### Final Phase: Mark the Implementation Complete

This is the **last** phase of the plan. Once its work and gates are finished,
and before your final summary, set these Frontmatter properties:

- on the plan ({{plan}}) **and** the implementation log ({{log}}):
    - `source_code`: every source code file created or updated across all phases of the plan
    - `documentation`: every documentation file created or updated across all phases of the plan
    - `completed_phase`: `{{phase}}`
    - `implemented`: `true`
::block when="spec"
- on the spec ({{spec}}):
    - `implemented`: `true`
    - `implemented_by`: "{{ctx.agent}}/{{ctx.model}}"
::end-block

The pipeline reads these flags to decide what runs next, so the plan is not
complete until they are saved to disk.
::end-block

**IMPORTANT:** 

::block when="ctx.area && has_skill(ctx.area)"
- use the '{{area}}' skill during the implementation
::block when="phase == total_phases"
- do NOT move the spec directory into the `_completed` folder when the final phase is complete (that is done as a separate step which you are not responsible for)
::end-block
::end-block
- Do NOT commit or stage files to git, this will be done as a separate process.
- Report a summary of what you did including all the source files you changed.
- You do not need to run tests across the entire monorepo as this will take far too long. Only run the gates for the '{{ctx.current_package_area}}' package area plus the dependent-package tests described in "Test Scope", and skip the gates only as described in "When the Test and Lint Gates May Be Skipped".
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!

::file "../_finish-before-exit.md"

::file "../_test-scope.md"

::file "../_os.md"
