---
start:
    - info: "starting"
    - when: "ctx.season == 'summer'"
      then:
          - info: "it is summer!"
          - shell: "run-summer-program"
      else:
          - info: "it is not summer"
    - message: "all is well that ends well"
    - stop
---
# depth 1
