---
$schema:
    initialize: stack@../../schemas/lifecycle-collapsed.yaml
    start: stack@../../schemas/lifecycle-collapsed.yaml
    blocked: stack@../../schemas/lifecycle-collapsed.yaml
    success: stack@../../schemas/lifecycle-collapsed.yaml
    failure: stack@../../schemas/lifecycle-collapsed.yaml
    finalize: stack@../../schemas/lifecycle-collapsed.yaml
    loop: loop@../../schemas/lifecycle-collapsed.yaml
success:
    - when: "ctx.level == 5"
      then:
          - when: "ctx.level == 4"
            then:
                - when: "ctx.level == 3"
                  then:
                      - when: "ctx.level == 2"
                        then:
                            - when: "ctx.level == 1"
                              then:
                                  - say: "leaf"
                              else:
                                  - warn: "level 1 false"
                        else:
                            - warn: "level 2 false"
                  else:
                      - warn: "level 3 false"
            else:
                - warn: "level 4 false"
      else:
          - warn: "level 5 false"
---
# variant
