---
sequence:
    - name: lint
      group:
          name: lint
          max_paralel: 2
          tasks:
              - { name: rust, shell: just lint }
---
Body.
