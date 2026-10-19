---
# 2026-10-24-sequence-containers: a removed `teardown:` rewritten as a group
sequence:
    - name: review
      group:
          name: review
          tasks:
              - name: review
                prompt: "@prompts/review.md"
          finalize:
              - when: "ctx.dirty_files"
                then:
                    - message: "I've made a mess of the place"
---
Body.
