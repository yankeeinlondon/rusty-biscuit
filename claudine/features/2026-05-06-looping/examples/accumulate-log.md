---
loop:
  while: "iteration < 5"
  actions:
    - "append(log, {\"iteration\": {{iteration}}, \"output\": \"{{last_output}}\"})"
log: ""
---

This prompt demonstrates accumulating a JSONL log across iterations.

Iteration: {{iteration}}
Last output: {{last_output}}

Current log:
{{log}}
