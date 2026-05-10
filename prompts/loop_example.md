---
iteration: 1
start:
    stderr: "We're on iteration {{iteration}}"
loop:
    until: "iteration > 3"
    actions: increment(iteration)
---

Hi. What do you think about the number {{iteration}}.
