---
area: "{{ctx.current_package_area}}"
dir: ""
spec: "spec.md"
design: "design.md"
plan: "plan.md"
iteration: 1
review: "review-{{iteration}}.md"
success:
    stderr: "Feature review {{iteration}} completed"
    message: "Feature review {{iteration}} for {{area}}/{{dir}} completed"
failure:
    message: "Feature review {{iteration}} for {{area}}/{{dir}} failed"
---
# Review Feature

Review the completed feature implementation against:

- Specification: `{{area}}/{{dir}}/{{spec}}`
- Design: `{{area}}/{{dir}}/{{design}}`
- Plan: `{{area}}/{{dir}}/{{plan}}`

## Review Focus

- Missing or incomplete functionality.
- Behavioral regressions.
- Incorrect assumptions relative to the specification or design.
- Insufficient test coverage.
- Lint, formatting, or maintainability issues that should block production readiness.
- Documentation or skill updates that were required but missed.

## Closure

- Save the review to `{{area}}/{{dir}}/{{review}}`.
- Set frontmatter property `ready` to `true` only if the feature is production-ready.
- Set frontmatter property `ready` to `false` if any implementation, test, lint, documentation, or workflow issue remains.
- Include actionable findings with file and line references where possible.
- Do not modify implementation files.
- Do not commit or stage files.
- Do not ask the user for feedback or permission; this is a non-interactive session.
