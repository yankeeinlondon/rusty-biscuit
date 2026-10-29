---
initialize:
  - append_line: ["trace.log", "event seq:initialize"]
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
  - name: checks
    group:
      name: checks
      max_parallel: 1
      tasks:
        - name: leg
          sequence:
            - { name: leg-work, shell: "printf 'task leg\\n' >> trace.log" }
          initialize:
            - append_line: ["trace.log", "event leg:initialize"]
            - retry: 1
          start:
            - append_line: ["trace.log", "event leg:start"]
          success:
            - append_line: ["trace.log", "event leg:success"]
          blocked:
            - append_line: ["trace.log", "event leg:blocked"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          failure:
            - append_line: ["trace.log", "event leg:failure"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          cancelled:
            - append_line: ["trace.log", "event leg:cancelled"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          finalize:
            - append_line: ["trace.log", "event leg:finalize"]
            - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
      initialize:
        - append_line: ["trace.log", "event checks:initialize"]
      start:
        - append_line: ["trace.log", "event checks:start"]
      success:
        - append_line: ["trace.log", "event checks:success"]
      blocked:
        - append_line: ["trace.log", "event checks:blocked"]
        - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
      failure:
        - append_line: ["trace.log", "event checks:failure"]
        - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
      cancelled:
        - append_line: ["trace.log", "event checks:cancelled"]
        - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
      finalize:
        - append_line: ["trace.log", "event checks:finalize"]
        - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
  - name: later
    shell: "printf 'task later\\n' >> trace.log"
---
