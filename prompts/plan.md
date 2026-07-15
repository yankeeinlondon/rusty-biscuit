---
$schema:
    - spec: file(required;match(**/*spec*.md);eager) -> path to specification file
      design: file(match(**/*design*.md)) -> path to the design file (if exists)
      plan: "file(required;match(**/*plan*.md)) -> The _plan file_ this prompt will create"
    - review: "file(required;match(**/*review.md);eager) -> if the plan we are building is based on a review (_instead of a `spec`_)"
      plan: "file(required;match(**/*plan*.md)) -> The _plan file_ this prompt will create"
    
description: "Creates a multi-phase, high confidence plan from a _feature_ or _fix_"
underlying: {{ spec || review }}
underlying_name: '{{ spec ? "spec" : "review" }}'
plan: "{{ dirname(spec || review) + '/plan.md' }}"
start:
    message: "🖊️ creating a plan for the `{{underlying}}` {{underlying_name}}"
success:
    stderr: "The `{{link(plan)}}` _plan_ has been created"
    message: "✅  the _plan_ for the spec `{{parent_dir(plan)}}` was created _at_ {{ctx.time}}"
failure:
    message: "❌️  the _plan_ for the {{underlying_name}} `{{underlying}}` failed to complete!" 
---

You are a planning agent. Convert the following documents into a high confidence execution plan:

::block when="spec"
- Functional Specification: {{spec}}
::end-block
::block when="design"
- Technical Design: {{design}}
::end-block
::block when="review"
- Review: {{review}}
::end-block

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
    - `total_phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `phase` set this to the starting phase number; usually 1 but may be 0 sometimes
    - `agent` set this to "{{ ctx.agent }}/{{ ctx.model }}"
    - `yolo` set this to "{{ env.YOLO }}"
