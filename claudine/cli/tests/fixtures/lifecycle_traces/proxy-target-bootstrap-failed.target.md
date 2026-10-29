---
initialize:
  - append_line: ["trace.log", "event target:initialize"]
  - shell: "printf 'shell target:initialize\\n' >> trace.log"
start:
  - append_line: ["trace.log", "event target:start"]
success:
  - append_line: ["trace.log", "event target:success"]
blocked:
  - append_line: ["trace.log", "event target:blocked"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
failure:
  - append_line: ["trace.log", "event target:failure"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
finalize:
  - append_line: ["trace.log", "event target:finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
---
Target
