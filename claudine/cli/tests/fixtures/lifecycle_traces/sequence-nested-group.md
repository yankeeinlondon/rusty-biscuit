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
finalize:
  - append_line: ["trace.log", "event seq:finalize"]
  - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
sequence:
  - name: first
    prompt: ./sequence-nested-group.step.md
  - name: checks
    group:
      name: checks
      max_parallel: 1
      tasks:
        - name: leg-a
          sequence:
            - { name: leg-a-work, shell: "printf 'task leg-a\\n' >> trace.log" }
          initialize:
            - append_line: ["trace.log", "event leg-a:initialize"]
          start:
            - append_line: ["trace.log", "event leg-a:start"]
          success:
            - append_line: ["trace.log", "event leg-a:success"]
          blocked:
            - append_line: ["trace.log", "event leg-a:blocked"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          failure:
            - append_line: ["trace.log", "event leg-a:failure"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          finalize:
            - append_line: ["trace.log", "event leg-a:finalize"]
            - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
        - name: leg-b
          sequence:
            - { name: leg-b-work, shell: "printf 'task leg-b\\n' >> trace.log" }
          initialize:
            - append_line: ["trace.log", "event leg-b:initialize"]
          start:
            - append_line: ["trace.log", "event leg-b:start"]
          success:
            - append_line: ["trace.log", "event leg-b:success"]
          blocked:
            - append_line: ["trace.log", "event leg-b:blocked"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          failure:
            - append_line: ["trace.log", "event leg-b:failure"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          finalize:
            - append_line: ["trace.log", "event leg-b:finalize"]
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
      finalize:
        - append_line: ["trace.log", "event checks:finalize"]
        - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
  - name: last
    shell: "printf 'task last\\n' >> trace.log"
---
