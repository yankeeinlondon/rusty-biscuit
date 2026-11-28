---
fail_fast: null
sequence:
  - name: ask
    promt: ./other.md
    prompt: ./ask.md
    agent: claude
    owner: docs
  - name: checks
    group:
      fail_fast: false
      execution: parallel
      max_paralel: 2
      tasks:
        - { name: lïnt, shell: just lint, teardown: ./clean.md }
  - { name: ünï, shell: x, setup: y }
---
Body.
