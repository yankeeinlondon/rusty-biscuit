---
area: "{{ctx.current_package_area}}"
dir: "$(dirname {{ "{{spec}}" || "{{design}}" }})"
spec: ""
design: ""
plan: "plan.md"
success:
    stderr: "The **{{area}}/{{dir}}/{{plan}}** _plan_ has been completed"
    message: "✅ the **{{area}}/{{dir}}/{{plan}}** _plan_ has been completed _at_ {{ctx.time}}"
failure: 
    message: "❌ the **{{area}}/{{dir}}/{{plan}}** _plan_ has failed to complete!"
---
You are a planning agent. Convert the following documents into a high confidence execution plan:

::block when="spec"
- Functional Specification: {{ctx.current_package_area}}/{{spec}}
::end-block
::block when="design"
- Technical Design: {{ctx.current_package_area}}/{{design}}
::end-block

## Requirements

- Break work into phases and steps
- Order steps by dependency
- Flag parallelizable work
- Include validation checkpoints
- Keep steps concrete and observable

## Closure

- Save the plan as "{{ctx.repo_root}}/{{ctx.current_package_area}}/{{dir}}/{{plan}}" in the same directory as the design document(s).
- Add frontmatter to the plan document and set:
    - `phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `start_phase` set this to the starting phase number; usually 1 but may be 0 sometimes

