---
sequence:
    - name: outer
      sequence:
          - name: inner
            prompt: "@prompts/inner.md"
            with: { topic: parsing }
          - group:
                name: legs
                fail_fast: false
                cancelled: []
                tasks:
                    - { name: leg, shell: just leg }
---
Body.
