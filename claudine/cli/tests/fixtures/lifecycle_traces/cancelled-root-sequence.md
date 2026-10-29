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
  - name: interrupt
    shell: "printf 'task interrupt\\n' >> trace.log; parent=$(ps -o comm= -p $PPID); case \"$parent\" in *claudine*) kill -INT $PPID ;; *) printf 'task wrong-parent %s\\n' \"$parent\" >> trace.log; exit 99 ;; esac; i=0; while [ $i -lt 400 ]; do i=$((i+1)); sleep 0.01; done; printf 'task not-cancelled\\n' >> trace.log"
  - name: later
    shell: "printf 'task later\\n' >> trace.log"
---
