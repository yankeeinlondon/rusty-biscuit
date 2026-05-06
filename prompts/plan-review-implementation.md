---
review: ""
root: "{{ctx.repo_root}}"
area: "{{ctx.current_package_area}}"
dir: "$(dirname review)"
plan: "plan.md"
plan_filepath: "{{area}}/{{dir}}/{{plan}}"
parameters:
    review: "string"
    plan: 
        type: "string"
        required: false
start: 
    message: "🏃‍♂️ creating a plan from the review **{{area}}/{{review}}**; planning being done with {{env.AGENT}} agent "
success:
    message: "🖥️ A high confidence plan `{{area}}/{{plan}}` has been created from a review."
    stderr: "✅ A high confidence **plan** for the review `{{area}}/{{review}}` has been created." 
failure:
    message: "❌ failed to implement the _suggestions_ from the review `{{area}}/{{dir}}/review-{{iteration}}.md`"
---
## Context

You are a **senior-level Project Manager** with extensive experience in planning complex, multi-phase pieces of work that carefully and skillfully balance speed of execution with safety and structured and explicit ordering which


## Task

- use the skill '{{area}}'
- read the review located at '{{review}}'

If the review has no additional recommendations and the frontmatter property 'ready' is set to true:

- The review completed with NO suggestions; the '{{area}}' package area is already healthy and no plan is needed
- communicate to the user and exit

In almost all cases, however, the review will full of prioritized suggestions and your task to create a high confidence, multi-phased plan from these suggestions.

Once you've created and saved the plan to '{{plan_filepath}}':

- set the `agent` frontmatter property of the plan file to {{env.AGENT}}
- set the `phases` to the total number of phases (or total - 1 if the first phase is labelled Phase 0)
- set the `start_phase` frontmatter property to the first phase in the plan (usually 1 but occasionally 0)
- set the `created` frontmatter property as '{{ctx.now}}' 

