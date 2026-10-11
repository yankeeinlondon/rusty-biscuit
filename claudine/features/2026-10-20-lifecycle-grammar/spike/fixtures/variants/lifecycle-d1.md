---
$schema:
    initialize: stack@../../schemas/lifecycle-d1.yaml
    start: stack@../../schemas/lifecycle-d1.yaml
    blocked: stack@../../schemas/lifecycle-d1.yaml
    success: stack@../../schemas/lifecycle-d1.yaml
    failure: stack@../../schemas/lifecycle-d1.yaml
    finalize: stack@../../schemas/lifecycle-d1.yaml
    loop: loop@../../schemas/lifecycle-d1.yaml
success:
    - when: "ctx.level == 1"
      then:
          - say: "leaf"
      else:
          - warn: "level 1 false"
---
# variant
