---
area: "{{ctx.current_package_area}}"
dir: ""
spec: "spec.md"
design: "design.md"
plan: "plan.md"
success:
    stderr: "The implementation plan has been completed"
    message: "The implementation plan for {{area}}/{{dir}} has been completed"
failure:
    message: "The implementation plan for {{area}}/{{dir}} failed to complete"
---
# Plan Feature

You are a planning agent. Convert the feature documents into a high-confidence execution plan.

- Specification: `{{area}}/{{dir}}/{{spec}}`
- Design: `{{area}}/{{dir}}/{{design}}`

## Requirements

- Use the `{{ctx.current_package_area}}` skill while preparing the plan.
- Read the specification and design before writing the plan.
- Break work into ordered phases and concrete steps.
- State dependencies between phases.
- Mark work that can safely run in parallel.
- Include validation checkpoints for each phase.
- Keep steps observable and specific enough for an implementation agent to execute without clarification.

## Closure

- Save the plan to `{{area}}/{{dir}}/{{plan}}`.
- Add frontmatter to `{{area}}/{{dir}}/{{plan}}` with:
    - `phases`: total number of phases in the plan
    - `created`: today's date in `YYYY-MM-DD` format
    - `start_phase`: `1`
- Do not commit or stage files.
