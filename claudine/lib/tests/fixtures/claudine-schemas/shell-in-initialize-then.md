---
initialize:
    - when: "ctx.offline"
      then:
          - shell: "git fetch"
# `agent` makes this a Claudine document; `initialize` alone is shared with md.
agent: claude
---
# shell-in-initialize-then
