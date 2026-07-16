---
operation: "merge-conflict"
conflicts: {{ ctx.merge_conflicts || null }}

initialize: 
    stack:
        when: "!conflicts"
        action:
            - info: |-
                There are **no** merge conflicts in this branch! If you want to "predict" what files will be conflict when you _do_
                merge then execute the `conflict-forecast.md` instead.
            - stop
---
## Context

You are in the **{{ctx.branch}}** of the **{{ctx.repo}}** repo. A merge has recently been performed and there are the following merge conflicts:

{{ conflicts }}

## File Type Tips

- `.vscode/settings.json`: often just a case of both branches having added new words to the "cSpell.words" dictionary. In these cases you should include a unique list of all words added across the two branches
- `.zed/settings.json`: 
    - treat conflicts as "additive" where possible (e.g., if both branches mutated different _keys_ in the JSON then accept both)
    - if the key with the conflict is an array or object the we should try to be "additive" at this level unless there is a good reason not to be
        - if we're taking an "additive" approach to array entries always be sure that the new array is a _unique_ list of items
    - if there is any other form of conflict at the key level it's almost always the "newer" version which should be kept



## Task

- Create a plan for resolving these conflicts.
- When the plan is ready, execute the plan. 
- Once the plan has been completed summarize the conflicts and how they were resolved.
