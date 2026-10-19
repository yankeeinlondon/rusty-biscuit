---
initialize:
    - when: "ctx.level == 1"
      then:
          - when: "ctx.level == 2"
            then:
                - when: "ctx.level == 3"
                  then:
                      - shell: "git fetch"
---
# shell-in-initialize-below-typed-levels
