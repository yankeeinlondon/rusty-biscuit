---
start:
    - say: "{{ 1 + }}"
    - info: { text: 'hello {{ 1 + }}' }
    - set: { round: '{{ fm.round + }}', listing: '$(ls)' }
    - shell: 'echo {{ 2 * }}'
    - when: "fm.round"
      then:
          - error: { reason: 'failed {{ 3 - }}' }
      else:
          - proxy: { target: next.md, with: { note: 'next {{ 4 / }}' } }
---
# handwritten-action-malformed-template
