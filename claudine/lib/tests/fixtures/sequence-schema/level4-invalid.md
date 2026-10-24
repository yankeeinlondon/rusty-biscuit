---
# Containers recurse: a fourth-level task and a fifth-level group are typed
# like the first.
sequence:
    - name: l1
      sequence:
          - name: l2
            sequence:
                - name: l3
                  sequence:
                      - name: l4
                        with: [not, a, mapping]
                        sequence:
                            - name: l5
                              group: { name: g, execution: parallel, tasks: [{ name: l6 }] }
---
Body.
