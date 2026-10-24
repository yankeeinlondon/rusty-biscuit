---
initialize:
    - when: "ctx.level == 4"
      then:
          - when: "ctx.level == 3"
            then:
                - when: "ctx.level == 2"
                  then:
                      - when: "ctx.level == 1"
                        then:
                            - info: "initialize then"
                        else:
                            - info: "initialize else"
start:
    - when: "ctx.level == 4"
      then:
          - when: "ctx.level == 3"
            then:
                - when: "ctx.level == 2"
                  then:
                      - when: "ctx.level == 1"
                        then:
                            - ensure_file: { file: notes.md, content: "# Notes" }
                        else:
                            - pr_list: { count: 3 }
cancelled:
    - when: "ctx.level == 4"
      then:
          - when: "ctx.level == 3"
            then:
                - when: "ctx.level == 2"
                  then:
                      - when: "ctx.level == 1"
                        then:
                            - stop
                        else:
                            - warn: "cancelled else"
loop:
    while: "iteration < max_iterations"
    action: increment(iteration)
    gate:
        - when: "ctx.level == 4"
          then:
              - when: "ctx.level == 3"
                then:
                    - when: "ctx.level == 2"
                      then:
                          - when: "ctx.level == 1"
                            then:
                                - info: "gate then"
                            else:
                                - ensure_file: notes.md
---
# depth 4 completion

Each event nests four conditionals; the innermost `then` and `else` are the
completion and hover positions.
