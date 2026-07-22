---
$schema:
    spec: file(required;eager; match(**/*spec*.md)) -> the specification file that defines the target goal for this feature or fix
    design: file(match(**/*design*.md))
    iteration: number(required) -> the iteration of the review cycle
    skip_performance_gates: boolean -> allows you to skip findings in the review which are due to a performance gate having not been met (or proven)
    review: file -> the latest review iteration which we will use to create the plan
    plan: file(required;eager;match(**/*plan*.md))
    phase: number(required) -> the phase of the plan we are implementing
    total_phases: number
    
iteration: |-
    {{
        frontmatter(spec, 'review_iterations')
            ? frontmatter(spec, 'review_iterations') || 1 
            : 1
    }}
phase: "{{ file_exists(plan) ? frontmatter(plan, 'phase') || 1 : null }}"
total_phases: "{{ file_exists(plan) ? frontmatter(plan, 'total_phases') || frontmatter(plan, 'phases') || 1 : 0 }}"
design: "{{ file_exists(replace(spec, 'spec', 'design')) ? file_exists(replace(spec, 'spec', 'design')) : null"
review: {{ dirname(spec) + '/' + 'review-' + iteration + '.md' }}
plan: {{ dirname(spec) + '/' + 'review-plan-' + iteration + '.md' }}
log: "{{ replace(spec, 'review', 'log') }}"
start:
    message: "👓  implementing phase **{{phase}}** `{{review}}`"
success:
    message: "🎉  completed the plan for the review findings in `{{review}}`"
failure:
    message: "💥  creating a plan for the review findings of `{{review}}` failed [{{err.code}}, {{err.msg}}]"
loop:
    until: "phase >= total_phases"
    action: "increment(phase)"
---
::block when="total_phases"
# Implement Phase {{phase}} of {{total_phases}}
::end-block
::block when="!total_phases"
# Implement Phase {{phase}}

> ⚠️ there was no `total_phases` set on this plan! This metadata missing may indicate a problem
::end-block

## Context

The plan you are implementing is composed of _findings_ from the review "{{review}}". This review was
conducted to determine how well the current implementation meets the requirements of:

- Specification File: {{spec}}
::block when="design && file_exists(design)"
- Design File: {{design}}
::end-block

## Task

Your task is to implement phase {{phase}} of the plan found in '@{{plan}}'.

- check off tasks in the plan -- marked by GFM todos (aka., `[ ]`) -- once they are complete
    - don't wait until the end of the phase
    - marking tasks complete in real time allows graceful recovery of the implementation of the plan if anything were to go wrong with the initial implementation of the plan

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
- You must set the following Frontmatter properties:
    - `source_files_during_phase_{{phase}}` should be set to all source code files which were created or updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_updated_during_phase_{{phase}}` should be set to all documentation files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_created_during_phase_{{phase}}` should be set to all documentation files which were created during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `skills_files_updated_during_phase_{{phase}}` should be set to all agent skill files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    ::block when="phase == total_phases"
        - set `source_code` Frontmatter to every source code file that was updated or created during the various phases of the plan
        - set `documentation` Frontmatter to every documentation file that was updated or created during the various phases of the plan
    ::end-block
    - if this is a monorepo, then include `packages` as a list of packages in the monorepo which were touched by the implementation in phase {{phase}}

::block when="ctx.is_monorepo"
## Be Efficient in Testing/Building

- when building or testing, make sure to only build/test the _specific packages_ or package area you are working; not the entire monorepo (this will take too long)
- The session was started in the "{{ctx.area}}" package area and so that's very likely an area you'll be focused on, however, 
- most plan's will have a `packages` or `blast_radius` Frontmatter property which will explicitly state which packages are in the "blast radius" (aka, will be impacted 
  during the implementation of this plan)
::end-block

**IMPORTANT:** 

::block when="has_skill(ctx.area)"
- use the '{{ctx.area}}' skill during the implementation
::block when="phase == total_phases"
- do NOT move the spec directory into the `_completed` folder when the final phase is complete (that is done as a separate step which you are not responsible for)
::end-block
::end-block
- Do NOT commit or stage files to git, this will be done as a separate process.
- Report a summary of what you did including all the source files you changed.
- You do not need to run tests across the entire monorepo as this will take far too long. Only 
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
