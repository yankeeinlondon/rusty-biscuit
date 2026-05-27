---
$schema:
    - review: string(required)
      spec: string
      iteration: number
      has_plan: bool
      has_spec: bool
      has_review: bool
    - spec: string(required)
      has_plan: bool
      has_spec: bool
      has_review: bool
    - plan: string(required)
      spec: string
      iteration: number
      has_plan: bool
      has_spec: bool
      has_review: bool
name: Implement Review Suggestions
description: |-
    Implements either:

    1. all the recommendations/suggestions produced in a review (pass in `review=...`)
    2. all the phases of a plan (pass in `plan=...`)
    3. if passed just a specification, it will:
        - implement if simple
        - create a plan for the implementation of the plan
iteration: 1
area: "{{ ctx.current_package ? ctx.current_package : ctx.current_package_area }}"
has_spec: "spec ? true : false"
has_plan: "plan ? true : false"
has_review: "review ? true : false"
dir: "$(dirname '{{spec || plan}}')"

plan_file: "$({{has_plan}} ? $(basename '{{plan}}' : '' }} )"
spec_file: "$({{has_spec}} ? $(basename '{{spec}}'  : '' }} )"
review_file: "$({{has_review}} ? basename '{{spec}}'  : '' )"

spec_path: "@{{area}}/{{dir}}/{{spec_file}}"
plan_file: "@{{area}}/{{dir}}/{{plan_file}}"
review_path: "@{{area}}/{{dir}}/{{review_file}}"

init: 
    stack:
        - when: has_plan
          action: 
            proxy: prompts/implement-plan.md
            phase: phase
            total_phases: total_phases
            plan: plan
---
::block when="review"
## Context

- Use the '{{area}}' agent skill when reviewing
- This task is focused on the '{{area}}' package area which has the following packages:

    ::shell sniff repo packages --package-area "{{ctx.current_package_area}}" --md

You will **implement** all of the suggestions found in the review:

- {{review_path}}

::block when="iteration == 1"
> Note: this is the first attempt at implementing the review's suggestions
::end-block
::block when="has_spec"

A prior review of the _implementation_ of the specification did NOT deem the implementation
to be "production ready" but we have now implemented all of the suggestions from that review
and your task will be to again compare the implementation of the specification relative to
the written intention of the specification.

> Note: you should _also_ validate that all of the "complaints/suggestions" of the prior review have been fully addressed. You are current performing review #{{iteration}} so you should be looking for review in the @{{area}}/{{dir}} directory with a name similar to "review-{{iteration - 1}}.md"
::end-block
::block when="!has_spec"
Prior attempts at implementing the review findings were deemed incomplete. This is the #{{iteration}} attempt to complete this review's findings.
::end-block

::file _test-rigor.md

## Task

::block when="has_spec && !has_plan"

::block when="has_review"
We are responsible for implementing all of the suggestions found in a review that was run on the
specification:

- {{spec_path}}

The review was run 

::end-block
::block when="!has_review"
The functionality we will be implementing is defined by the specification file: {{spec_path}}.

- if the specification has complexity to it that would benefit from a high confidence, multi-phase plan then:
    - create a high confidence plan and save it to "@{{area}}/{{dir}}/plan-${iteration}.md"
    - the plan: 
        - will be run in a non-interactive session; be aware of this in the planning stage and avoid permission requests which might be blocked in a non-interactive session
        - the first phase of the plan should always be phase 1 (no exceptions)
        - each phase will be implemented serially one after the other
        - tasks in each phase should be marked as GFM todo's so that during implementation the agent can check them off as they progress through the work
        - if there are concurrency opportunities then specify them _inside_ a phase not _across_ phases
    - once the plan has been saved, we will add the following frontmatter properties to the plan file:
        - `created` set to '{{ctx.now}}'
        - `spec` set to '{{spec_path}}'
        - `total_phases` set to the total number of phases the plan has
        - `phase` set to `1`
    - since the initial intent of the caller was that you "implement" the spec and so far what you did was take a pre-step of "planning for the implementation":
        - let them know that you've created the plan and provide an OSC8 link to the plan
        - let them know that they can implement the plan by running: `claudine compose @prompts/implement-plan.md plan={plan} total_phases={total_phases} phase=1`
- if the specification is a fairly simple task then we don't need to go to the extra burden of creating a plan first:
    - communicate to the caller:
        - let them know that the specification is a simple task and will be implemented without the need for a separate plan
        - implement the specification and make sure test coverage meets the **Test Rigor** standards defined above
        - you are only done when all tests pass and there are no lint errors/warnings in the package area that you're working
            - it doesn't matter if you feel these errors/warnings weren't caused by your code
            - in most cases the package area will start clean (from a git perspective) and all tests/lints will be passing so in most cases IT WILL BE YOUR CODE
            - but if for some reason there were some errors that crept in, it's important that they immediately be closed out too (unless the user explicitly says otherwise)
::end-block
::end-block

## Closure

- Save your review suggestions to "{{review_path}}"
- based on your review suggestions indicate whether you think this feature is **ready for production** by setting the `ready` frontmatter property on "{{review_path}}" to `true` or `false`
- save the `agent` frontmatter property as "{{env.AGENT}}" in the "{{review_path}}" file
- save the `model` frontmatter property as "{{env.MODEL}}" in the "{{review_path}}" file

## **IMPORTANT:**

- do NOT change the `ready` property in the review file after implementing
    - you may feel that everything in that review was fixed but the review's assessment at that time should not change
    - furthermore, we will be running another review _after_ you've completed here to validate that everything is fixed
- do not run `cargo fmt` ... we want functional changes during this work not formatting changes
- do not commit your work to git (this will be done as an independent process which you are not responsible for)
::file ./you-are-non-interactive.md
- communicate as much as possible so that the caller can keep track of progress

::end-block

::block when="!spec && review"

The following review has just completed:

- {{review_path}}

::block when="iteration != 1"
A prior review did NOT deem the implementation to be "production ready" but we have now implemented all of the suggestions from that review and your task will be to again compare the implementation of the specification relative to the written intention of the specification.

> Note: you should also validate that all of the "complaints/suggestions" of the _prior_ review have now been fully addressed. You are current performing review #{{iteration}} so you should be looking for review in the 
{{dir}} directory with a name similar to "review-{ {{iteration}} - 1 }.md"
::end-block

Your task is to implement all the suggestions in that review.

::end-block
