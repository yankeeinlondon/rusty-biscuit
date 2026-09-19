---
$schema: ./implement.yaml
description: |-
    provides instruction on what properties to set in the Frontmatter for an implementation's log file; each implementation prompt is responsiblee for setting:

    - `log` the log file being used to document progress
    - `kind` the _kind_ of implementation this is
---
The implementation log file -- {{log}} -- which you have used to document the progress made during this implementation should now have a well structured prose report in the body of the log file; now we need to add in :

- `implemented_by` set to `{{ctx.agent}}/{{ctx.model}}`
::block when="spec"
- `spec` set to `{{spec}}
::end-block
::block when="design"
- `design` set to `{{design}}
::end-block
::block when="review"
- `review` set to `{{review}}`
::end-block

::block when="kind == 'initial-spec-implementation'"
Since this was
::end-block
