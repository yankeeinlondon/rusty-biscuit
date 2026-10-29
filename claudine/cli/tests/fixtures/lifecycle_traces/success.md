---
counter: 0
initialize:
  - append_line: ["trace.log", "event initialize"]
  - set: { counter: 1 }
  - append_line: ["trace.log", "state counter={{ counter }}"]
start:
  - append_line: ["trace.log", "event start"]
  - shell: "printf 'shell start\\n' >> trace.log"
success:
  - append_line: ["trace.log", "event success"]
  - set: { counter: 2 }
  - append_line: ["trace.log", "state counter={{ counter }}"]
blocked:
  - append_line: ["trace.log", "event blocked"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
failure:
  - append_line: ["trace.log", "event failure"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
finalize:
  - append_line: ["trace.log", "event finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
  - append_line: ["trace.log", "state counter={{ counter }}"]
  - shell: "printf 'shell finalize\\n' >> trace.log"
---
Hello
