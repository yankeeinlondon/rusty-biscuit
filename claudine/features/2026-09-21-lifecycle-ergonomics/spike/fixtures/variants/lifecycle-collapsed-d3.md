---
$schema:
    initialize: stack@../../schemas/lifecycle-collapsed-d3.yaml
    start: stack@../../schemas/lifecycle-collapsed-d3.yaml
    blocked: stack@../../schemas/lifecycle-collapsed-d3.yaml
    success: stack@../../schemas/lifecycle-collapsed-d3.yaml
    failure: stack@../../schemas/lifecycle-collapsed-d3.yaml
    finalize: stack@../../schemas/lifecycle-collapsed-d3.yaml
    loop: loop@../../schemas/lifecycle-collapsed-d3.yaml
success:
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
---
# variant
