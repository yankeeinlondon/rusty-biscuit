---
dir: ""
spec: ""
design: ""
iteration: 1
success:
    say: "Feature review {{iteration}} has completed"
failure:
    say: "Feature review {{iteration}} failed to complete!"
---

We have just completed a feature defined in "{{dir}}":

- the specification file is located at: "{{dir}}/{{spec}}"
- the technical design is located at: "{{dir}}/{{design}}"

Read both the specification and design documents and then perform a review on the implementation:

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality which is light on test coverage (we expect strong unit and integration testing for everything)
- are there any changes which would make the code more ergonomic, more performant, or both?

::block when="iteration == 1"
Save your review suggestions to "{{dir}}/review.md"
::end-block

::block when="iteration != 1"
Save your review suggestions to "{{dir}}/review-{{iteration}}.md"
::end-block


**IMPORTANT:**

::block when="ctx.current_package_area"
- use the '{{ctx.current_package_area}}' skill during the implementation
::end-block
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
