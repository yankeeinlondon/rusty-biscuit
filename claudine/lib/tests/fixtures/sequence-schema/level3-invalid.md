---
sequence:
    - name: a
      sequence:
          - name: b
            sequence:
                - name: c
                  prompt: "@prompts/c.md"
                  with: [1]
                - group:
                      name: legs
                      execution: parallel
                      tasks:
                          - { name: leg, shell: just leg }
---
Body.
