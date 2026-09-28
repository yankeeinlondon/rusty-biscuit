---
$schema:
    initialize: stack@../../schemas/lifecycle-d2.yaml
    start: stack@../../schemas/lifecycle-d2.yaml
    blocked: stack@../../schemas/lifecycle-d2.yaml
    success: stack@../../schemas/lifecycle-d2.yaml
    failure: stack@../../schemas/lifecycle-d2.yaml
    finalize: stack@../../schemas/lifecycle-d2.yaml
    loop: loop@../../schemas/lifecycle-d2.yaml
success:
    - when: "ctx.level == 2"
      then:
          - when: "ctx.level == 1"
            then:
                - say: "leaf"
            else:
                - warn: "level 1 false"
      else:
          - warn: "level 2 false"
---
# variant
