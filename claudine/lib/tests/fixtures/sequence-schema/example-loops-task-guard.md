---
# 2026-10-25-loops-and-flow-control: a task guard and task inputs
sequence:
    - name: implement
      prompt: "@prompts/_implement/implement-plan.md"
      when: "frontmatter(spec, 'implemented') != true"
      with:
          spec: "{{ spec }}"
---
Body.
