---
agent: claude
start: [{ info: starting }]
sequence:
    - name: one
      prompt: "@prompts/one.md"
      with: "{{ payload }}"
---
Body.
