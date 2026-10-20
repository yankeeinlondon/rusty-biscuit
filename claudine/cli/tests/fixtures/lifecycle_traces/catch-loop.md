---
loop:
  until: "false"
  max: 2
  gate:
    - append_line: ["trace.log", "event loop"]
    - when: "frontmatter('nope-loop.md', 'status') == 'x'"
      then:
        - append_line: ["trace.log", "unreachable"]
initialize:
  - append_line: ["trace.log", "event initialize"]
start:
  - append_line: ["trace.log", "event start"]
  - append_line: ["trace.log", "state loop.count={{ loop.count }}"]
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
