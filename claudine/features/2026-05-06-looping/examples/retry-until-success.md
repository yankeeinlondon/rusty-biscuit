---
loop:
  until: "last_exit_code == 0"
  fail_fast: false
  actions:
    - "set(attempted, true)"
    - "increment(retries)"
retries: 0
attempted: false
---

This prompt demonstrates a retry loop that continues until success.

Iteration: {{iteration}}
Retries so far: {{retries}}
Last exit code: {{last_exit_code}}
