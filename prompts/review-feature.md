---
dir: "$(pwd)"
spec: ""
design: ""
iteration: 1
area: "{{ctx.current_package_area}}"
start: "🏃‍♂️ starting the feature review of `{{dir}}` -- in the **{{ctx.current_package_area}}** _package area_ -- _at_ {{ctx.now}}"
success:
    stderr: "Feature review {{iteration}} in the {{ctx.current_package_area}} package area has completed"
    message: "✅ feature review {{iteration}} in the **{{ctx.current_package_area}}** package area has completed:\nSpecification: {{dir}}/{{spec}}\nDesign: {{dir}}/{{design}}\n\nThe review can be found at: {{area}}/{{dir}}/review-{{iteration}}.md"
failure:
    stderr: "Feature review {{iteration}} in the {{ctx.current_package_area}} package area failed to complete!"
    message: "❌ feature review {{iteration}} in the {{ctx.current_package_area}} package area failed to complete!"
---

We have just completed a feature defined in "{{area}}/{{dir}}":

::block when="spec"
- specification: "{{area}}/{{dir}}/{{spec}}"
::end-block
::block when="design"
- technical design: "{{area}}/{{dir}}/{{design}}"
::end-block

::block when="And(spec, design)"
Read both the specification and design documents and then perform a review on the implementation:
::end-block
::block when="spec"
Read both the specification document and then perform a review on the implementation:
::end-block
::block when="design"
Read both the specification document and then perform a review on the implementation:
::end-block

::block when="iteration != 1"
> **Note:** this is _not_ the first review we've done on this functionality but the prior review's
> suggestions have now all been implemented.

::end-block

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality which is light on test coverage (we expect strong unit and integration testing for everything)
- are there any changes which would make the code more ergonomic, more performant, or both?

## Closure

- Save your review suggestions to "{{area}}/{{dir}}/review-{{iteration}}.md"
- based on your review suggestions indicate whether you think this feature is ready for production by setting the `ready` frontmatter property on "{{area}}/{{dir}}/review-{{iteration}}.md"

**IMPORTANT:**

::block when="ctx.current_package_area"
- use the '{{ctx.current_package_area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
