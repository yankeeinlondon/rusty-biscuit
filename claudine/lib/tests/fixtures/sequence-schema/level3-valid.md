---
sequence:
    - name: a
      sequence:
          - name: b
            group:
                name: legs
                tasks:
                    - name: c
                      sequence:
                          - { name: d, prompt: "@prompts/d.md", with: { x: 1 } }
                    - name: e
                      group:
                          name: inner
                          max_parallel: 1
                          tasks:
                              - { name: f, shell: just f }
---
Body.
