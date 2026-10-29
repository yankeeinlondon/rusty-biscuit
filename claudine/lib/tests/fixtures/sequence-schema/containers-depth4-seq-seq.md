---
sequence:
- name: l1
  sequence:
  - name: l2
    sequence:
    - name: l3
      sequence:
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
