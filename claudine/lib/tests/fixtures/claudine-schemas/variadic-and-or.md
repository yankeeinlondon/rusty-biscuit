---
start:
    - and
    - and: [true, "{{ fm.ready }}", false]
    - or: []
    - or: ["{{ fm.ready }}"]
---
# variadic-and-or
