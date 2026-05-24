---
loop:
  while: "counter < 5"
  action: "increment(counter)"
counter: 0
---

This is iteration {{_loop_count}} of the loop.

Current counter value: {{counter}}
Is first: {{_loop_is_first}}
Is last: {{_loop_is_last}}
