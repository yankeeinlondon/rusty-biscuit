---
# 2026-10-24-sequence-containers: sibling cancellation (the group, as a step)
sequence:
    - group:
          name: fan-out
          fail_fast: true
          tasks:
              - name: slow-review
                prompt: "@prompts/review.md"   # declares `cancelled:`, so it is cancelled
              - name: quick-check
                shell: just check              # no `cancelled:`; runs to completion
          failure:
              - warn: "failed: {{ as_csv(group.failed) }}; cancelled: {{ as_csv(group.cancelled) }}"
---
Body.
