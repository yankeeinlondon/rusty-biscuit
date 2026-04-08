---
dir: ""
spec: ""
design: ""
iteration: 1
success:
    say: "Review {{iteration}} has completed"
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
