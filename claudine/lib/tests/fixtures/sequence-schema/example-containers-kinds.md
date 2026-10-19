---
# 2026-10-24-sequence-containers: the three container kinds
sequence:
    - name: build
      sequence:                       # inline nested sequence
          - name: compile
            shell: just build
          - name: test
            shell: just test
    - name: release-notes
      sequence: "@sequences/notes.md" # nested sequence by file reference
    - name: lint-all
      group:                          # concurrent legs
          name: lint
          tasks:
              - name: rust
                shell: just lint
              - name: docs
                shell: just lint-docs
---
Body.
