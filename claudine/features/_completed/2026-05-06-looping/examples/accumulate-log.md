---
loop:
  while: "_loop_count < 5"
  action:
    - "append(log, {\"iteration\": {{_loop_count}}, \"output\": \"{{_loop_last_output}}\"})"
log: ""
---

This prompt demonstrates accumulating a JSONL log across iterations.

Iteration: {{_loop_count}}
Last output: {{_loop_last_output}}

Current log:
{{log}}
