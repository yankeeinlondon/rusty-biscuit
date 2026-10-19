---
# 2026-10-24-sequence-containers: a tolerant group and its finalize event
sequence:
    - name: optional-docs
      group:
          name: optional-docs
          fail_fast: false             # this step's failure does not stop the run
          tasks:
              - name: docs
                prompt: "@prompts/docs.md"
          finalize:
              - when: "length(group.failed) > 0"
                then:
                    - warn: "docs step failed; continuing"
    - name: release
      prompt: "@prompts/release.md"
---
Body.
