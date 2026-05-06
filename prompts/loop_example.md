---
iteration: 1
start:
    stderr: "We're on iteration {{iteration}}"
loop:
    until: "iteration == 3"
    action: increment(iteration)
---

Hi
