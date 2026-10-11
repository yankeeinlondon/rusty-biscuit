---
$schema:
    initialize: stack@../../schemas/lifecycle-d4.yaml
    start: stack@../../schemas/lifecycle-d4.yaml
    blocked: stack@../../schemas/lifecycle-d4.yaml
    success: stack@../../schemas/lifecycle-d4.yaml
    failure: stack@../../schemas/lifecycle-d4.yaml
    finalize: stack@../../schemas/lifecycle-d4.yaml
    loop: loop@../../schemas/lifecycle-d4.yaml
success:
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
---
# variant
