---
loop:
  while: "counter < 5"
  actions: "increment(counter)"
counter: 0
---

This is iteration {{iteration}} of the loop.

Current counter value: {{counter}}
Is first: {{is_first}}
Is last: {{is_last}}
