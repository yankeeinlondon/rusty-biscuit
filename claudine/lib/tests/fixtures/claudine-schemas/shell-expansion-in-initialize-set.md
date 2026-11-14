---
initialize:
    - set:
          sha: "$(git rev-parse HEAD)"
# `agent` makes this a Claudine document; `initialize` alone is shared with md.
agent: claude
---
# shell-expansion-in-initialize-set
