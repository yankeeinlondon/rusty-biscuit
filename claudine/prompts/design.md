---
area: "{{ctx.current_package_area}}"
dir: ""
spec: "spec.md"
design: "design.md"
success:
    stderr: "The design document has been completed"
    message: "The design document for {{area}}/{{dir}}/{{spec}} has been completed"
failure:
    message: "The design document for {{area}}/{{dir}}/{{spec}} failed to complete"
---
# Design Feature

You are a senior technical design agent. Produce a focused technical design for the feature described by:

- Specification: `{{area}}/{{dir}}/{{spec}}`

## Requirements

- Use the `{{ctx.current_package_area}}` skill while preparing the design.
- Read the specification before writing the design.
- Save the design to `{{area}}/{{dir}}/{{design}}`.
- Make the design idiomatic Markdown using CommonMark and GFM.
- Reference the specification file in the body of the design.
- Do not duplicate long sections of the specification. Add implementation detail, architecture, testing strategy, edge cases, and risk notes.
- Use mermaid code blocks only when a diagram makes dependencies or flow materially clearer.

## Closure

- Ensure `{{area}}/{{dir}}/{{design}}` exists.
- Ensure the document body is non-empty.
- Do not commit or stage files.
