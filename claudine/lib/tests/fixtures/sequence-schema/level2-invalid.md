---
sequence:
    - name: outer
      sequence:
          - group:
                name: legs
                execution: parallel
                tasks:
                    - { name: leg, shell: just leg }
---
Body.
