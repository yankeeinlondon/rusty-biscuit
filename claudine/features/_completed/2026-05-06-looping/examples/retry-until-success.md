---
loop:
  until: "_loop_last_exit_code == 0"
  fail_fast: false
  action:
    - "set(attempted, true)"
    - "increment(retries)"
retries: 0
attempted: false
---

This prompt demonstrates a retry loop that continues until success.

Iteration: {{_loop_count}}
Retries so far: {{retries}}
Last exit code: {{_loop_last_exit_code}}
