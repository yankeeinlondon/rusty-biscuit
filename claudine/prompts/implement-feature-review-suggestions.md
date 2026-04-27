---
area: "{{ctx.current_package_area}}"
dir: ""
spec: "spec.md"
design: "design.md"
plan: "plan.md"
review: ""
iteration: 1
success:
    stderr: "Review suggestions {{iteration}} have been implemented"
    message: "Review suggestions {{iteration}} for {{area}}/{{dir}} have been implemented"
failure:
    message: "Review suggestions {{iteration}} for {{area}}/{{dir}} failed"
---
# Implement Review Suggestions

Implement the changes requested by the feature review.

- Specification: `{{area}}/{{dir}}/{{spec}}`
- Design: `{{area}}/{{dir}}/{{design}}`
- Plan: `{{area}}/{{dir}}/{{plan}}`
- Review: `{{area}}/{{dir}}/{{review}}`
- Review iteration: `{{iteration}}`

## Requirements

- Use the `{{ctx.current_package_area}}` skill during implementation.
- Read the review before editing code.
- Apply all actionable review suggestions unless a suggestion is demonstrably obsolete because the code already satisfies it.
- Add or update tests for the reviewed behavior.
- Keep edits scoped to the review suggestions.
- Do not commit or stage files.
- Run `just test` in the `{{ctx.current_package_area}}` package area.
- Run `just lint` in the `{{ctx.current_package_area}}` package area.

## Closure

- Report every source file changed.
- Report every documentation or skill file changed.
- Report test and lint results.
- If any review suggestion was intentionally not applied, explain why.
- Do not ask the user for feedback or permission; this is a non-interactive session.
