---
dir: ""
spec: ""
design: ""
iteration: 1
---

## Context

A review was just conducted in the {{ctx.current_package_area}} package area, the core documents were:

- specification "{{dir}}/{{spec}}"
- technical design document "{{dir}}/{{design}}"

## Task

::block when="iteration == 1"
Implement all of the suggestions found in review found at "{{dir}}/review.md"
::end-block

::block when="iteration != 1"
Implement all of the suggestions found in review found at "{{dir}}/review-{{iteration}}.md"
::end-block

**IMPORTANT:**

- use the '{{ctx.current_package_area}}' skill during the implementation
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
