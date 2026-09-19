---
description: |-
    Consolidates the memory file used for git commits
memory: .claudine/memory/commits.md
prompt: prompts/commit.md
---
## Context

The "commits" memory file -- {{ memory }} -- accumulates novel observations and suggestions to help make committing to git as successful as possible. Over time this list of lessons learned can become large.

## Task

- review the commit prompt ({{prompt}})
    - understand all of the key outcomes we are trying to achieve
- review the memory file ({{memory}}) for commits
    - eliminate any duplication that exists in the memory file
    - do a thorough analysis of each remaining suggestion to make sure that:
        - it is a valid suggestion
        - it helps to achieve the goals defined in the commit prompt
    - remove any suggestions that feel duplicative with whatever is already stated in the commit prompt or which is not valid

You are done once both prompt and memory file are reviewed and updated.

> Note: it's ok not to make any changes so long as there is no redundancy nor rules which are invalid or counterproductive
