---
$schema:
    - spec: file(required;match(**/*spec*.md);eager) -> path to specification file
      design: file(match(**/*design*.md)) -> path to the design file (if exists)
      plan: "file(required;match(**/*plan*.md)) -> The _plan file_ this prompt will create"
      area: "string() -> the package area (or package in some cases) where the work is being done"
    - review: "file(required;match(**/*review.md);eager) -> if the plan we are building is based on a review (_instead of a `spec`_)"
      plan: "file(required;match(**/*plan*.md)) -> The _plan file_ this prompt will create"
      area: "string() -> the package area (or package in some cases) where the work is being done"
    
description: "Creates a multi-phase, high confidence plan from a _feature_ or _fix_"
root: "{{ctx.repo_root}}"
area: "{{ctx.area }}"
plan: "{{ dirname(spec) + '/plan.md' }}"
start:
    message: "🖊️ creating a plan for the `{{spec}}` specification"
success:
    stderr: "The <blue>{{link(plan)}}</blue> plan has been created"
    message: "✅  the plan for the spec `{{parent_dir(spec)}}` _in_ **{{ctx.area}}** was created _at_ {{ctx.time}}"
failure:
    message: "❌️  the plan for the spec `{{parent_dir(spec)}}` _in_ **{{ctx.area}}** failed to complete!"
---

You are a planning agent. Convert the following documents into a high confidence execution plan:

::block when="spec"
- Functional Specification: {{ctx.current_package_area}}/{{spec}}
::end-block
::block when="design"
- Technical Design: {{ctx.current_package_area}}/{{design}}
::end-block
::block when="review"

## Requirements

- Break work into **phases** and **tasks**
- Order tasks by dependency
- Flag parallelizable work
- Include validation checkpoints
- Keep tasks concrete and observable
- the tasks in the plan lead with a GFM inspired todo marker (e.g., `- [ ] {task}`)
    - this allows the implementation team to check off items in the plan as they complete them
- plans should ALWAYS start with Phase 1 (not Phase 0 or something else non-standard)

## Closure

- Save the plan as "{{plan}}"
- Add frontmatter to the plan document and set:
    - `agent` set this to "{{env.AGENT}}"
    - `total_phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `phase` set this to the starting phase number; usually 1 but may be 0 sometimes
    - `agent` set this to "{{ env.AGENT }}/{{ env.MODEL || default }}"
    - `yolo` set this to "{{ env.YOLO }}"
