---
sequence:
    - name: one
      prompt: "@prompts/one.md"
      with: { topic: parsing }
      when: "ready"
    - group:
          name: pair
          max_parallel: 2
          loop: { until: "done", max: 3 }
          cancelled: [stop]
          tasks:
              - { name: left, shell: just left }
              - { name: right, shell: just right }
---
Body.
