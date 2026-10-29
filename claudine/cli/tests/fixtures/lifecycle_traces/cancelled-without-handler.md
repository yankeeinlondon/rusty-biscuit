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
      fail_fast: true
      tasks:
        - name: opted
          cancelled: []
          sequence:
            - name: inner
              sequence:
                - name: inner-work
                  shell: "printf 'task inner waiting\\n' >> trace.log; : > opted.ready; i=0; while [ ! -e never.released ]; do i=$((i+1)); [ $i -lt 400 ] || exit 90; sleep 0.01; done; printf 'task inner completed\\n' >> trace.log"
              initialize:
                - append_line: ["trace.log", "event inner:initialize"]
              start:
                - append_line: ["trace.log", "event inner:start"]
              success:
                - append_line: ["trace.log", "event inner:success"]
              blocked:
                - append_line: ["trace.log", "event inner:blocked"]
                - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
              failure:
                - append_line: ["trace.log", "event inner:failure"]
                - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
              finalize:
                - append_line: ["trace.log", "event inner:finalize"]
                - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
          initialize:
            - append_line: ["trace.log", "event opted:initialize"]
          start:
            - append_line: ["trace.log", "event opted:start"]
          success:
            - append_line: ["trace.log", "event opted:success"]
          blocked:
            - append_line: ["trace.log", "event opted:blocked"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          failure:
            - append_line: ["trace.log", "event opted:failure"]
            - append_line: ["trace.log", "err {{ err.code || 'uncoded: ' + err.msg }}"]
          finalize:
            - append_line: ["trace.log", "event opted:finalize"]
            - append_line: ["trace.log", "err {{ err ? (err.code || 'uncoded: ' + err.msg) : 'none' }}"]
        - name: bad
          shell: "i=0; while [ ! -e opted.ready ]; do i=$((i+1)); [ $i -lt 400 ] || exit 91; sleep 0.01; done; printf 'task bad\\n' >> trace.log; exit 1"
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
