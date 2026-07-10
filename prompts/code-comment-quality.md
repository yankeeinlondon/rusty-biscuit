---
name: Code Comment Quality
description: |-
    This prompt will analyze the source code in a package area and make sure it conforms to the best practices
    that this monorepo recommends.
favorite: true
area: "{{ ctx.current_package ? ctx.current_package : ctx.current_package_area }}"
operation: "code-comments"
---

## Best Practices

::file @docs/code-quality.md 

## Lessons Learned

Read the best practices but before you execute the task, read the "lessons learned" that have been accumulated
while performing the "code-comments" task so that you can benefit from others who have executed this process:

{{ctx.repo_root}}/.claudine/memory/code-comments.md

## Source Code Scope

This task is about improving the quality of the code comments in source code but this is a large monorepo and you
should not tackle all of it in one go. Instead YOUR focus is exclusively on the following packages:

{{ as_unordered_list(ctx.current_packages) }}
