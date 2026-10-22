---
initialize:
  - append_line: ["trace.log", "event initialize"]
  - proxy: "./proxy-in-initialize.target.md"
start:
  - append_line: ["trace.log", "event start"]
success:
  - append_line: ["trace.log", "event success"]
blocked:
  - append_line: ["trace.log", "event blocked"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
failure:
  - append_line: ["trace.log", "event failure"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
finalize:
  - append_line: ["trace.log", "event finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
---
Hello
