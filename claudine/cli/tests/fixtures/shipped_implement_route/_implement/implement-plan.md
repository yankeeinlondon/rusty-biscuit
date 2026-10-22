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
    - ensure_file: "{{log}}"
    - when: "!total_phases || total_phases <= 0"
      then:
          - warn: "The plan `{{plan}}` does not provide metedata on how many _phases_ the plan has!"
          - message: "The plan `{{plan}}` does not provide metedata on how many _phases_ the plan has!"
          - error: "for a plan to be implemented using the **implement-plan** prompt, you need to ensure the plan ({{plan}}) has set either `total_phases` or `phases` Frontmatter property!"
    # Decisions that need the author are settled before any phase runs, so a
    # spec, plan, or paused log still waiting on one fails here, not after a
    # phase has spent its work. The `message` stays short for chat routes; the
    # full items go to the terminal.
    - when: "spec && (frontmatter(spec, 'human_review') == true || frontmatter(spec, 'needs_rulings') == true)"
      then:
          - message: "🤷  cannot implement `{{parent_dir(plan)}}`: its spec has **{{ length(frontmatter(spec, 'human_review_items') || frontmatter(spec, 'required_rulings') || []) }}** decision(s) waiting on the author"
          - warn: |-
                The spec {{link(spec)}} has decisions that need the author before it can be implemented:

                {{ as_ordered_list(frontmatter(spec, 'human_review_items') || frontmatter(spec, 'required_rulings') || []) }}

                Settle them (for example with the **clarify** prompt), record them in the spec, set its `human_review` and `needs_rulings` to `false`, and run the implement prompt again.
          - error: "`{{parent_dir(plan)}}` cannot be implemented until the author settles its spec's open decisions"
    - when: "file_exists(plan) && frontmatter(plan, 'human_review') == true"
      then:
          - message: "🤷  cannot implement `{{parent_dir(plan)}}`: its plan has **{{ length(frontmatter(plan, 'human_review_items') || []) }}** decision(s) waiting on the author"
          - warn: |-
                The plan {{link(plan)}} has decisions that need the author before it can be implemented:

                {{ as_ordered_list(frontmatter(plan, 'human_review_items') || []) }}

                Record your decisions in the plan's `### Decisions`, set its `human_review` to `false`, and run the implement prompt again.
          - error: "`{{parent_dir(plan)}}` cannot be implemented until the author settles its plan's open decisions"
    - when: "file_exists(log) && frontmatter(log, 'human_review') == true"
      then:
          - message: "🤷  cannot resume `{{parent_dir(plan)}}`: an earlier phase paused for **{{ length(frontmatter(log, 'human_review_items') || []) }}** decision(s) that are still waiting on the author"
          - warn: |-
                An earlier phase of {{link(plan)}} paused for decisions that are still waiting on the author:

                {{ as_ordered_list(frontmatter(log, 'human_review_items') || []) }}

                Record your decisions in {{link(log)}}, set its `human_review` to `false`, and run the implement prompt again.
          - error: "`{{parent_dir(plan)}}` cannot resume until the author settles the decisions an earlier phase paused for"
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
    # The agent writes the completion flags; this only surfaces a final
    # phase that exited without them, since marking them here could claim
    # gates the agent never finished.
    - when: "phase >= total_phases && (!frontmatter(plan, 'implemented') || (spec && !frontmatter(spec, 'implemented')))"
      then:
        - warn: "the final phase of `{{parent_dir(plan)}}` finished without setting `implemented: true` on the plan and spec; check the log before treating it as complete"
        - message: "⚠️  final phase of `{{parent_dir(plan)}}` finished without `implemented: true` on the plan and spec; check the log before treating it as complete"
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
    until: "phase >= total_phases"
    action: "increment(phase)"
    # A phase that hit a surprise it could not decide pauses the plan (rare by
    # design; see the prompt's "Handoff to the Next Phase"). The non-zero code
    # stops a chained `&& just fix`, which would otherwise only report that
    # the spec is not implemented. The `message` stays short for chat routes.
    gate:
        - when: "frontmatter(log, 'human_review') == true"
          then:
              - warn: |-
                    implementation of `{{parent_dir(plan)}}` **paused after phase {{phase}}** (_of {{total_phases}}_): the agent found something during the implementation that needs a human decision before the next phase can start. Review items:

                    {{ as_ordered_list(frontmatter(log, 'human_review_items') || []) }}

                    Record your decisions in {{link(log)}}, set its `human_review` to `false`, and run the implement prompt again.
              - message: "🤷  implementation of `{{parent_dir(plan)}}` paused after phase **{{phase}}** (_of {{total_phases}}_) for **{{ length(frontmatter(log, 'human_review_items') || []) }}** human decision(s) ({{area}}, {{ctx.agent}}/{{ctx.model}}); see `human_review_items` in its implementation log"
              - say: "The plan in the {{area}} {{ctx.area_description}} paused after phase {{phase}} and needs your review"
              - break:
                    reason: "phase {{phase}} of {{total_phases}} requires human review before the plan can continue"
                    code: 2
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

::file "../_decisions.md"

This phase's decisions go in the log under `## Phase {{phase}}` → `### Design Decisions`.
The spec, the plan's `### Decisions`, and any decision files the plan points to
were settled before this implementation started: implement their
recommendations as written, even where a file still describes them as
"pending" or "needing a ruling".

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
- name the boundary the behavior depends on and choose the fidelity rung (F1/F2/F3) and tier
required by `rust-testing`; unit coverage alone
is insufficient when behavior crosses crate, CLI, filesystem, terminal, or
persistence boundaries
- log important details of your work to {{log}} under the `## Phase {{phase}}` section

When changing parsers, schemas, templates, prompts, or configuration-driven
behavior:

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

- set `human_review` to `false`, except in the rare case below
- set `human_review` to `true` **only** when this phase uncovered a fact nobody knew when the spec and plan were written (for example, an assumption the plan depends on is false in the code), that fact meets the bar in "When a human is needed", and the next phase cannot proceed on any recommendation you can make
    - an ambiguity, a criterion that cannot be met literally, a recommendation already in the spec or plan, or a choice between reasonable options is **not** a reason: decide it and record it
    - when you set it, also set `human_review_items`: one YAML `|-` block per item, written without jargon for a reader who does not know this repo, giving what you found, why the next phase cannot proceed without a decision, the options with pros and cons, and your recommendation
    - the plan pauses after this phase, and its next run refuses to start until the author clears the flag
::block when="phase == total_phases"
    - this is the final phase, so do not use `human_review` here; a critical decision found now goes in `decisions_for_author` (see "Pipeline Refresh and Critical Decisions" below)
::end-block
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

### Pipeline Refresh and Critical Decisions

This implementation may be one step in a pipeline of changes: the spec has a
`pipeline_step` frontmatter property, or another active spec (one not under
`_completed` or `_unscheduled`) names this spec's `{date}-{name}` in its
`depends-on`. The specs and plans later in that pipeline were written before
this implementation, so check them against what you now know:

- find the downstream specs: those with a higher `pipeline_step` (the pipeline's order is also listed in the document the spec's `pipeline_step` description names), and those whose `depends-on` names this spec; skip any already `implemented: true`
- read each downstream spec and its plan (when it has one) against this implementation: the code as it now is, the decisions in {{log}}, and anything that turned out differently from what those documents assume
- update them directly; you do not need anyone's approval. Correct wrong facts, symbol names, and file paths; adopt the approach this implementation established; remove work this implementation already delivered; and record new decisions they now need, following "Design Decisions" above
    - in a plan that has started, change only phases that have not started
    - never drop or narrow an outcome a downstream spec asks for
    - add a short entry to each changed document's decisions section that names this spec by its `{date}-{name}` and says what changed and why
- record the result in {{log}} under `## Pipeline Refresh`, one line per downstream spec (changed and why, or checked and unchanged), and set the log's `pipeline_refreshed` frontmatter to the `{date}-{name}` of every spec or plan you changed (an empty list when none)
- if this spec is not part of a pipeline, write that in one line under `## Pipeline Refresh` and set `pipeline_refreshed` to `[]`

A **critical** decision, in this implementation or in a downstream spec that
this implementation's facts now force, meets the bar in "When a human is
needed". It is rare. List each one in the log's `decisions_for_author`
frontmatter (one YAML `|-` block per item, written without jargon, with the
options, pros and cons, and your recommendation), and still finish the phase
and the refresh using your recommendation. The first review of this
implementation reads that list and opens an interactive session with the
author to settle it. Set `decisions_for_author` to `[]` when there are none.
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
