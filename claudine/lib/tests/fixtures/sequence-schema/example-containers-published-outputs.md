---
# 2026-10-24-sequence-containers: published outputs of nested containers
sequence:
    - name: A
      shell: echo "A out"
    - name: inner
      sequence:
          - { name: X, shell: echo "X out" }
          - { name: Y, shell: echo "Y out" }
    - name: checks
      group:
          name: checks
          tasks:
              - { name: lint, shell: echo "lint out" }
              - { name: test, shell: echo "test out" }
    - name: B
      shell: echo "B out"
---
Body.
