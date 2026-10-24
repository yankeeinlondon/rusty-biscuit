---
loop:
    until: "{{ true }}"
    ensure_file: { file: review.txt }
    no_error: true
---
# single-action-loop-loose
