---
start:
    - info: "loop start"
loop:
    while: "iteration < max_iterations"
    action: increment(iteration)
    gate:
        - effect: "tick"
        - when: "iteration == max_iterations"
          then:
              - warn: "iteration cap reached"
        - info: "finished iteration {{ iteration }} of {{ max_iterations }}"
---
# loop gate
