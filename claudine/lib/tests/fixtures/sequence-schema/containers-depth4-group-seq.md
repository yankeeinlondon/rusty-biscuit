---
sequence:
- name: l1
  group:
    name: g1
    tasks:
    - name: l2
      sequence:
      - name: l3
        group:
          name: g3
          tasks:
          - name: l4
            sequence:
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
