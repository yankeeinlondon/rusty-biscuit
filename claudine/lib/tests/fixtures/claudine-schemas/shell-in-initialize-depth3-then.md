---
initialize:
    - when: "ctx.level == 1"
      then:
          - when: "ctx.level == 2"
            then:
                - when: "ctx.level == 3"
                  then:
                      - shell: "git fetch"
# `agent` makes this a Claudine document; `initialize` alone is shared with md.
agent: claude
---
# shell-in-initialize-depth3-then
