---
dir: ""
spec: "spec.md"
success: 
    say: "The fix in the {{ctx.current_package_area}} package area has completed successfully"
error:
    say: "There was a problem in fixing the specified fix item in the {{ctx.current_package_area}} package area"
---

## Problem

::file {{dir}}/{{spec}}

## Task

- read the problem and make any required fixes
- then consider if there are any documents in the "{{ctx.current_package_area}}" package area that need to be updated as part of your change
- update any documents that need updating
- summarize your fix to the user
- save the following frontmatter to the file "{{dir}}/{{spec}}"
    - `fixed` should be today's date in YYYY-MM-DD format
    - `agent` should be set to "${env.AGENT}"
