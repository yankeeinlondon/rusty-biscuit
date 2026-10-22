---
initialize:
  - append_line: ["trace.log", "event seq:initialize"]
  - retry: 1
start:
  - append_line: ["trace.log", "event seq:start"]
success:
  - append_line: ["trace.log", "event seq:success"]
blocked:
  - append_line: ["trace.log", "event seq:blocked"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
failure:
  - append_line: ["trace.log", "event seq:failure"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
cancelled:
  - append_line: ["trace.log", "event seq:cancelled"]
  - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
finalize:
  - append_line: ["trace.log", "event seq:finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
sequence:
  - name: only
    shell: "printf 'task only\\n' >> trace.log"
---
