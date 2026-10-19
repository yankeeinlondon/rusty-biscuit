---
sequence:
    - name: lint
      group:
          name: lint
          execution: parallel
          tasks:
              - { name: rust, shell: just lint }
---
Body.
