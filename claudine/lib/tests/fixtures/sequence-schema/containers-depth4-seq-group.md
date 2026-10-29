---
sequence:
- name: l1
  sequence:
  - name: l2
    group:
      name: g2
      tasks:
      - name: l3
        sequence:
        - name: l4
          group:
            name: g4
            tasks:
            - name: leaf
              shell: echo leaf
            initialize:
            - info: initialize
            start:
            - info: start
            blocked:
            - info: blocked
            success:
            - info: success
            failure:
            - info: failure
            cancelled:
            - stop
            finalize:
            - info: finalize
---
Body.
