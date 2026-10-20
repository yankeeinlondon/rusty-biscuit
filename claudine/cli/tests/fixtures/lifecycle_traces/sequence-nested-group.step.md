---
initialize:
  - append_line: ["trace.log", "event step:initialize"]
start:
  - append_line: ["trace.log", "event step:start"]
success:
  - append_line: ["trace.log", "event step:success"]
blocked:
  - append_line: ["trace.log", "event step:blocked"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
failure:
  - append_line: ["trace.log", "event step:failure"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
finalize:
  - append_line: ["trace.log", "event step:finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
---
Step
