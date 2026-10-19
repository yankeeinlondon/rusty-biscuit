---
# Below the third container level a list is any list: the task parser, not
# the schema, judges level 4 and deeper.
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
