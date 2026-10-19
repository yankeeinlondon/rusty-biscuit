---
initialize:
    - when: "ctx.offline"
      then:
          - shell: "git fetch"
---
# shell-in-initialize-then
