---
initialize:
  - append_line: ["trace.log", "event target:initialize"]
start:
  - append_line: ["trace.log", "event target:start"]
  - shell: "printf 'shell target:start\\n' >> trace.log"
success:
  - append_line: ["trace.log", "event target:success"]
blocked:
  - append_line: ["trace.log", "event target:blocked"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
  - shell: "printf 'shell target:blocked\\n' >> trace.log"
failure:
  - append_line: ["trace.log", "event target:failure"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
finalize:
  - append_line: ["trace.log", "event target:finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
  - shell: "printf 'shell target:finalize\\n' >> trace.log"
---
Target

::file shell-relocked-after-proxy.missing.md
